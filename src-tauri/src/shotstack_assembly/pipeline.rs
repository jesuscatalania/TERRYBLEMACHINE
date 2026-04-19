//! Production [`VideoAssembler`] that drives [`ShotstackClient`] directly.
//!
//! Unlike [`crate::mesh_pipeline`] / [`crate::image_pipeline`] / [`crate::video_pipeline`]
//! we do **not** route through [`AiRouter`](crate::ai_router::AiRouter): the
//! Shotstack timeline JSON is provider-specific, there is no fallback
//! (no other provider speaks the same shape), and the caller hands us typed
//! [`AssemblyClip`] values that we already know how to serialise. Routing
//! would buy nothing and force us to pack/unpack the timeline through a
//! generic `AiRequest.payload`.
//!
//! The distinguishing feature vs. the client is the MP4-download-to-cache
//! step: Shotstack returns a remote CDN URL once rendering completes, but
//! Remotion preview / Remotion composition re-render benefits from a local
//! file so HTTP 200 first-paint latency is zero. We hash the remote URL
//! (sha256) and store at
//! `~/Library/Caches/terryblemachine/assemblies/<hash>.mp4`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::api_clients::shotstack::ShotstackClient;

use super::types::{AssemblyClip, AssemblyError, AssemblyInput, AssemblyResult, VideoAssembler};

pub struct ShotstackAssembler {
    client: Arc<ShotstackClient>,
    http: Client,
}

impl ShotstackAssembler {
    pub fn new(client: Arc<ShotstackClient>) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("reqwest client builds");
        Self { client, http }
    }

    /// Build the Shotstack timeline body from typed input. Mirrors the shape
    /// Shotstack documents for its Edit API:
    ///
    /// ```json
    /// { "timeline": { "tracks": [ { "clips": [ { "asset": { "type": "video", "src": "..." }, "start": 0, "length": 3 } ] } ] },
    ///   "output":   { "format": "mp4", "resolution": "hd" } }
    /// ```
    ///
    /// Transitions collapse to an optional `transition: { in, out }` object
    /// per clip; absent values stay absent. An `AssemblyInput.soundtrack` lands
    /// as a top-level `timeline.soundtrack.src`.
    pub fn build_timeline_body(input: &AssemblyInput) -> Value {
        let clips: Vec<Value> = input.clips.iter().map(Self::clip_to_json).collect();

        let mut timeline = json!({ "tracks": [ { "clips": clips } ] });
        if let Some(sound) = &input.soundtrack {
            timeline["soundtrack"] = json!({ "src": sound });
        }
        json!({
            "timeline": timeline,
            "output": { "format": input.format, "resolution": input.resolution }
        })
    }

    fn clip_to_json(c: &AssemblyClip) -> Value {
        let mut clip = json!({
            "asset": { "type": "video", "src": c.src },
            "start": c.start_s,
            "length": c.length_s,
        });
        if c.transition_in.is_some() || c.transition_out.is_some() {
            let mut t = json!({});
            if let Some(v) = &c.transition_in {
                t["in"] = json!(v);
            }
            if let Some(v) = &c.transition_out {
                t["out"] = json!(v);
            }
            clip["transition"] = t;
        }
        clip
    }

    fn cache_dir() -> Result<PathBuf, AssemblyError> {
        let base = dirs::cache_dir()
            .ok_or_else(|| AssemblyError::Cache("no platform cache dir".into()))?;
        let dir = base.join("terryblemachine").join("assemblies");
        std::fs::create_dir_all(&dir).map_err(|e| AssemblyError::Cache(e.to_string()))?;
        Ok(dir)
    }

    pub(crate) fn cache_path(remote_url: &str) -> Result<PathBuf, AssemblyError> {
        let mut h = Sha256::new();
        h.update(remote_url.as_bytes());
        let hash = format!("{:x}", h.finalize());
        let dir = Self::cache_dir()?;
        Ok(dir.join(format!("{hash}.mp4")))
    }

    /// Download `remote_url` into the cache, returning the local path. A
    /// cache hit (file already exists) short-circuits without re-fetching —
    /// Shotstack URLs are content-addressed via render ID, so identical
    /// URLs are safe to cache indefinitely.
    ///
    /// `file://` URLs are special-cased so integration tests can exercise
    /// the full pipeline without wiring a mock HTTP server.
    async fn download_to_cache(&self, remote_url: &str) -> Result<PathBuf, AssemblyError> {
        let path = Self::cache_path(remote_url)?;
        if path.exists() {
            return Ok(path);
        }

        if let Some(stripped) = remote_url.strip_prefix("file://") {
            let src = Path::new(stripped);
            tokio::fs::copy(src, &path)
                .await
                .map_err(|e| AssemblyError::Download(e.to_string()))?;
            return Ok(path);
        }

        let bytes = self
            .http
            .get(remote_url)
            .send()
            .await
            .map_err(|e| AssemblyError::Download(e.to_string()))?
            .bytes()
            .await
            .map_err(|e| AssemblyError::Download(e.to_string()))?;
        tokio::fs::write(&path, &bytes)
            .await
            .map_err(|e| AssemblyError::Cache(e.to_string()))?;
        Ok(path)
    }
}

#[async_trait]
impl VideoAssembler for ShotstackAssembler {
    async fn assemble(&self, input: AssemblyInput) -> Result<AssemblyResult, AssemblyError> {
        if input.clips.is_empty() {
            return Err(AssemblyError::InvalidInput("clips list is empty".into()));
        }

        let timeline_body = Self::build_timeline_body(&input);
        let render_id = self
            .client
            .assemble_timeline(timeline_body)
            .await
            .map_err(|e| AssemblyError::Provider(e.to_string()))?;
        let final_body = self.client.poll_render(&render_id).await.map_err(|e| {
            // FU #146: on timeout the render is still queued server-side and
            // will be billed even though we gave up waiting. Shotstack has no
            // public cancel endpoint at the time of writing, so the best we
            // can do is log the render id so the user can chase it on the
            // dashboard. Non-timeout errors get the same log line for
            // parity — they're rare and worth the trail.
            eprintln!(
                "[shotstack-assembler] poll failed for render {render_id}: {e} \
                 (render may still complete server-side and be billed)"
            );
            AssemblyError::Provider(e.to_string())
        })?;
        let video_url = final_body
            .pointer("/response/url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AssemblyError::Provider("shotstack: no url in done response".into()))?
            .to_string();
        // Best-effort download: a failed fetch falls back to `None` so the
        // frontend can still render from the remote URL. The failure is
        // logged so network issues don't disappear into the void.
        let local_path = match self.download_to_cache(&video_url).await {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!(
                    "[shotstack-assembler] download failed for {video_url}, \
                     falling back to remote URL: {e}"
                );
                None
            }
        };
        Ok(AssemblyResult {
            render_id,
            video_url,
            local_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(src: &str, start: f32, length: f32) -> AssemblyClip {
        AssemblyClip {
            src: src.into(),
            start_s: start,
            length_s: length,
            transition_in: None,
            transition_out: None,
        }
    }

    #[test]
    fn build_timeline_body_wraps_clips_in_single_track() {
        let input = AssemblyInput {
            clips: vec![clip("https://a/1.mp4", 0.0, 3.0)],
            soundtrack: None,
            format: "mp4".into(),
            resolution: "hd".into(),
        };
        let body = ShotstackAssembler::build_timeline_body(&input);
        let clips = body
            .pointer("/timeline/tracks/0/clips")
            .and_then(|v| v.as_array())
            .expect("clips path present");
        assert_eq!(clips.len(), 1);
        assert_eq!(
            clips[0].pointer("/asset/src").and_then(|v| v.as_str()),
            Some("https://a/1.mp4")
        );
        assert_eq!(
            clips[0].pointer("/asset/type").and_then(|v| v.as_str()),
            Some("video")
        );
        assert_eq!(
            body.pointer("/output/format").and_then(|v| v.as_str()),
            Some("mp4")
        );
        assert_eq!(
            body.pointer("/output/resolution").and_then(|v| v.as_str()),
            Some("hd")
        );
    }

    #[test]
    fn build_timeline_body_emits_transitions_only_when_present() {
        let input = AssemblyInput {
            clips: vec![
                AssemblyClip {
                    src: "a".into(),
                    start_s: 0.0,
                    length_s: 1.0,
                    transition_in: Some("fade".into()),
                    transition_out: None,
                },
                clip("b", 1.0, 1.0),
            ],
            soundtrack: None,
            format: "mp4".into(),
            resolution: "hd".into(),
        };
        let body = ShotstackAssembler::build_timeline_body(&input);
        let clips = body
            .pointer("/timeline/tracks/0/clips")
            .and_then(|v| v.as_array())
            .unwrap();
        assert_eq!(
            clips[0].pointer("/transition/in").and_then(|v| v.as_str()),
            Some("fade")
        );
        assert!(clips[0].pointer("/transition/out").is_none());
        assert!(
            clips[1].get("transition").is_none(),
            "absent transitions should not emit a transition key"
        );
    }

    #[test]
    fn build_timeline_body_emits_soundtrack_when_present() {
        let input = AssemblyInput {
            clips: vec![clip("a", 0.0, 1.0)],
            soundtrack: Some("https://cdn/music.mp3".into()),
            format: "mp4".into(),
            resolution: "hd".into(),
        };
        let body = ShotstackAssembler::build_timeline_body(&input);
        assert_eq!(
            body.pointer("/timeline/soundtrack/src")
                .and_then(|v| v.as_str()),
            Some("https://cdn/music.mp3")
        );
    }

    #[test]
    fn cache_path_is_deterministic_for_same_url() {
        let a = ShotstackAssembler::cache_path("https://cdn/r1.mp4").unwrap();
        let b = ShotstackAssembler::cache_path("https://cdn/r1.mp4").unwrap();
        let c = ShotstackAssembler::cache_path("https://cdn/r2.mp4").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.extension().map(|e| e == "mp4").unwrap_or(false));
    }
}
