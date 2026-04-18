//! End-to-end tests for [`RouterVideoPipeline`].
//!
//! Mirrors `mesh_pipeline_integration.rs` but exercises the MP4-download
//! step: we write a tiny fake MP4 to a tempdir, build a fake Kling client
//! that echoes back a `file://…` URL pointing at it, and let
//! `RouterVideoPipeline` pull it through the router → download path →
//! cache dir. The `file://` special-case inside `download_to_cache` lets
//! us verify the full pipeline without a mock HTTP server.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::json;
use tempfile::TempDir;

use terryblemachine_lib::ai_router::{
    AiClient, AiRequest, AiResponse, AiRouter, DefaultRoutingStrategy, Model, PriorityQueue,
    Provider, ProviderError, ProviderUsage, RetryPolicy,
};
use terryblemachine_lib::video_pipeline::{
    RouterVideoPipeline, VideoImageInput, VideoPipeline, VideoPipelineError, VideoTextInput,
};

/// Fake Kling client that echoes a predetermined `video_url` back inside
/// the router response. We use a `file://` URL so the pipeline's download
/// step stays local to this test. Captures the most recent `AiRequest` so
/// tests can assert routing inputs (task, payload, duration, …).
struct StubKlingClient {
    video_url_to_echo: String,
    last_request: Arc<Mutex<Option<AiRequest>>>,
}

#[async_trait]
impl AiClient for StubKlingClient {
    fn provider(&self) -> Provider {
        Provider::Kling
    }

    fn supports(&self, m: Model) -> bool {
        matches!(m, Model::Kling20)
    }

    async fn execute(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        *self.last_request.lock().unwrap() = Some(request.clone());
        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({
                "video_url": self.video_url_to_echo,
                "status": "succeeded",
            }),
            cost_cents: None,
            cached: false,
        })
    }

    async fn health_check(&self) -> bool {
        true
    }

    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
        Ok(ProviderUsage::default())
    }
}

#[allow(clippy::type_complexity)]
fn pipeline_with_capture(
    tmp: &TempDir,
) -> (
    RouterVideoPipeline,
    std::path::PathBuf,
    Arc<Mutex<Option<AiRequest>>>,
) {
    let fake_mp4 = tmp.path().join("fake.mp4");
    // Minimal MP4 magic bytes — test asserts cache round-trip, not parse.
    std::fs::write(&fake_mp4, b"\x00\x00\x00\x18ftypmp42").unwrap();
    let file_url = format!("file://{}", fake_mp4.display());

    let kling_capture: Arc<Mutex<Option<AiRequest>>> = Arc::new(Mutex::new(None));

    let mut clients: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();
    clients.insert(
        Provider::Kling,
        Arc::new(StubKlingClient {
            video_url_to_echo: file_url,
            last_request: Arc::clone(&kling_capture),
        }),
    );
    let router = Arc::new(AiRouter::new(
        Arc::new(DefaultRoutingStrategy),
        clients,
        RetryPolicy::default_policy(),
        Arc::new(PriorityQueue::new()),
    ));
    (RouterVideoPipeline::new(router), fake_mp4, kling_capture)
}

fn pipeline_with_mp4(tmp: &TempDir) -> (RouterVideoPipeline, std::path::PathBuf) {
    let (p, path, _capture) = pipeline_with_capture(tmp);
    (p, path)
}

#[tokio::test]
async fn text_to_video_downloads_to_cache() {
    let tmp = TempDir::new().unwrap();
    let (p, _src) = pipeline_with_mp4(&tmp);

    let r = p
        .generate_from_text(VideoTextInput {
            prompt: "a sunset over tokyo".into(),
            duration_s: Some(5.0),
            module: None,
        })
        .await
        .expect("text-to-video succeeds");

    assert!(
        r.video_url.contains("fake.mp4"),
        "video_url should echo stub URL, got {}",
        r.video_url
    );
    let local = r.local_path.expect("cache path present after download");
    assert!(local.exists(), "cached MP4 file should exist at {local:?}");
    assert!(
        local.extension().map(|e| e == "mp4").unwrap_or(false),
        "cache path should end in .mp4, got {local:?}"
    );
    assert_eq!(r.model, format!("{:?}", Model::Kling20));
    assert_eq!(
        r.duration_s,
        Some(5.0),
        "duration_s should be echoed from input"
    );
}

#[tokio::test]
async fn text_to_video_rejects_empty_prompt() {
    let tmp = TempDir::new().unwrap();
    let (p, _) = pipeline_with_mp4(&tmp);

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
async fn image_to_video_rejects_data_url() {
    let tmp = TempDir::new().unwrap();
    let (p, _) = pipeline_with_mp4(&tmp);

    let err = p
        .generate_from_image(VideoImageInput {
            image_url: "data:image/png;base64,abc".into(),
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
        "error should mention data-URLs, got: {err}"
    );
}

#[tokio::test]
async fn video_download_is_idempotent_across_calls() {
    let tmp = TempDir::new().unwrap();
    let (p, _) = pipeline_with_mp4(&tmp);

    let r1 = p
        .generate_from_text(VideoTextInput {
            prompt: "a horse running".into(),
            duration_s: Some(5.0),
            module: None,
        })
        .await
        .expect("first call succeeds");
    let r2 = p
        .generate_from_text(VideoTextInput {
            prompt: "a horse running".into(),
            duration_s: Some(5.0),
            module: None,
        })
        .await
        .expect("second call succeeds");

    // Both responses echo the same remote URL, so the content-addressed
    // cache path must match byte-for-byte.
    assert_eq!(
        r1.local_path, r2.local_path,
        "identical remote URL → identical cache path"
    );
}
