//! Production [`VideoPipeline`] that dispatches through [`AiRouter`] and
//! downloads the resulting MP4 into the platform cache directory.
//!
//! Mirrors [`crate::mesh_pipeline::pipeline`]: Kling (Runway/Higgsfield as
//! fallbacks, polling wired in T4) returns a remote MP4 URL, and the Tauri
//! webview benefits from loading it via a local `asset://` path to avoid
//! CORS/HTTP latency on first paint. We hash the remote URL (sha256), store
//! at `<cache-dir>/terryblemachine/videos/<hash>.mp4`, and return that path
//! for the frontend to pipe through `convertFileSrc`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::ai_router::{
    AiRequest, AiResponse, AiRouter, Complexity, Priority, RouterError, TaskKind,
};

use super::types::{
    VideoImageInput, VideoPipeline, VideoPipelineError, VideoResult, VideoTextInput,
};

pub struct RouterVideoPipeline {
    router: Arc<AiRouter>,
    http: Client,
}

impl RouterVideoPipeline {
    pub fn new(router: Arc<AiRouter>) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("reqwest client builds with default TLS");
        Self { router, http }
    }

    fn cache_dir() -> Result<PathBuf, VideoPipelineError> {
        let base = dirs::cache_dir()
            .ok_or_else(|| VideoPipelineError::Cache("no platform cache dir".into()))?;
        let dir = base.join("terryblemachine").join("videos");
        std::fs::create_dir_all(&dir).map_err(|e| VideoPipelineError::Cache(e.to_string()))?;
        Ok(dir)
    }

    fn cache_path(remote_url: &str) -> Result<PathBuf, VideoPipelineError> {
        let mut h = Sha256::new();
        h.update(remote_url.as_bytes());
        let hash = format!("{:x}", h.finalize());
        let dir = Self::cache_dir()?;
        Ok(dir.join(format!("{hash}.mp4")))
    }

    /// Download `remote_url` into the cache, returning the local path. A
    /// cache hit (file already exists) short-circuits without re-fetching —
    /// provider video URLs are task-ID-scoped, so identical URLs are safe
    /// to cache indefinitely.
    ///
    /// `file://` URLs are special-cased so integration tests can exercise
    /// the full pipeline without wiring a mock HTTP server.
    async fn download_to_cache(&self, remote_url: &str) -> Result<PathBuf, VideoPipelineError> {
        let path = Self::cache_path(remote_url)?;
        if path.exists() {
            return Ok(path);
        }

        if let Some(stripped) = remote_url.strip_prefix("file://") {
            let src = Path::new(stripped);
            tokio::fs::copy(src, &path)
                .await
                .map_err(|e| VideoPipelineError::Download(e.to_string()))?;
            return Ok(path);
        }

        let bytes = self
            .http
            .get(remote_url)
            .send()
            .await
            .map_err(|e| VideoPipelineError::Download(e.to_string()))?
            .bytes()
            .await
            .map_err(|e| VideoPipelineError::Download(e.to_string()))?;
        tokio::fs::write(&path, &bytes)
            .await
            .map_err(|e| VideoPipelineError::Cache(e.to_string()))?;
        Ok(path)
    }

    fn extract_video_url(resp: &AiResponse) -> Option<String> {
        resp.output
            .get("video_url")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    }
}

fn router_to_pipeline_err(err: RouterError) -> VideoPipelineError {
    VideoPipelineError::Router(err.to_string())
}

#[async_trait]
impl VideoPipeline for RouterVideoPipeline {
    async fn generate_from_text(
        &self,
        input: VideoTextInput,
    ) -> Result<VideoResult, VideoPipelineError> {
        if input.prompt.trim().is_empty() {
            return Err(VideoPipelineError::InvalidInput("prompt is empty".into()));
        }
        let mut payload = json!({});
        if let Some(d) = input.duration_s {
            payload
                .as_object_mut()
                .expect("payload is object")
                .insert("duration".into(), json!(d.round() as u64));
        }
        let req = AiRequest {
            id: uuid::Uuid::new_v4().to_string(),
            task: TaskKind::TextToVideo,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: input.prompt,
            payload,
        };
        let resp = self
            .router
            .route(req)
            .await
            .map_err(router_to_pipeline_err)?;
        let video_url = Self::extract_video_url(&resp).ok_or(VideoPipelineError::NoOutput)?;
        // Best-effort download: a failed fetch falls back to `None` so the
        // frontend can still render from the remote URL. Log the failure
        // (FU #146) so silent network issues surface in logs.
        let local_path = match self.download_to_cache(&video_url).await {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!(
                    "[video-pipeline] download failed for {video_url}, falling back to remote URL: {e}"
                );
                None
            }
        };
        Ok(VideoResult {
            video_url,
            local_path,
            model: format!("{:?}", resp.model),
            duration_s: input.duration_s,
        })
    }

    async fn generate_from_image(
        &self,
        input: VideoImageInput,
    ) -> Result<VideoResult, VideoPipelineError> {
        if input.image_url.trim().is_empty() {
            return Err(VideoPipelineError::InvalidInput(
                "image_url required".into(),
            ));
        }
        if input.image_url.starts_with("data:") {
            return Err(VideoPipelineError::InvalidInput(
                "video: hosted image URL required — data-URLs unsupported".into(),
            ));
        }
        let mut payload = json!({ "image_url": input.image_url });
        if let Some(d) = input.duration_s {
            payload
                .as_object_mut()
                .expect("payload is object")
                .insert("duration".into(), json!(d.round() as u64));
        }
        let req = AiRequest {
            id: uuid::Uuid::new_v4().to_string(),
            task: TaskKind::ImageToVideo,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: input.prompt.unwrap_or_default(),
            payload,
        };
        let resp = self
            .router
            .route(req)
            .await
            .map_err(router_to_pipeline_err)?;
        let video_url = Self::extract_video_url(&resp).ok_or(VideoPipelineError::NoOutput)?;
        let local_path = match self.download_to_cache(&video_url).await {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!(
                    "[video-pipeline] download failed for {video_url}, falling back to remote URL: {e}"
                );
                None
            }
        };
        Ok(VideoResult {
            video_url,
            local_path,
            model: format!("{:?}", resp.model),
            duration_s: input.duration_s,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::ai_router::{DefaultRoutingStrategy, PriorityQueue, RetryPolicy};

    fn zero_client_pipeline() -> RouterVideoPipeline {
        let router = Arc::new(AiRouter::new(
            Arc::new(DefaultRoutingStrategy),
            HashMap::new(),
            RetryPolicy::default_policy(),
            Arc::new(PriorityQueue::new()),
        ));
        RouterVideoPipeline::new(router)
    }

    #[tokio::test]
    async fn text_rejects_empty_prompt() {
        let p = zero_client_pipeline();
        let err = p
            .generate_from_text(VideoTextInput {
                prompt: "   ".into(),
                duration_s: None,
                module: None,
            })
            .await
            .expect_err("empty prompt must be rejected before routing");
        assert!(
            matches!(err, VideoPipelineError::InvalidInput(_)),
            "expected InvalidInput, got {err:?}"
        );
    }

    #[tokio::test]
    async fn image_rejects_data_url() {
        let p = zero_client_pipeline();
        let err = p
            .generate_from_image(VideoImageInput {
                image_url: "data:image/png;base64,iVBORw0KGgo=".into(),
                prompt: None,
                duration_s: None,
                module: None,
            })
            .await
            .expect_err("data-URL must be rejected before routing");
        assert!(
            matches!(err, VideoPipelineError::InvalidInput(_)),
            "expected InvalidInput, got {err:?}"
        );
        assert!(
            format!("{err}").contains("data-URLs"),
            "error message should mention data-URLs, got: {err}"
        );
    }

    #[test]
    fn cache_path_is_deterministic_for_same_url() {
        let a = RouterVideoPipeline::cache_path("https://fake/kling/123.mp4").unwrap();
        let b = RouterVideoPipeline::cache_path("https://fake/kling/123.mp4").unwrap();
        let c = RouterVideoPipeline::cache_path("https://fake/kling/456.mp4").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.extension().map(|e| e == "mp4").unwrap_or(false));
    }
}
