//! Kling AI client — text-to-video and image-to-video.
//!
//! Follows the reference pattern established in [`super::claude`] and the
//! polling loop from [`super::runway`]:
//! 1. `new(key_store)` + `with_base_url(..)` keychain-backed constructors.
//! 2. Impl of [`AiClient`](crate::ai_router::AiClient) dispatches both the
//!    `text2video` and `image2video` endpoints through `send_request` which
//!    POSTs, then polls `GET /v1/videos/{kind}/{task_id}` until Kling reports
//!    a terminal status.
//! 3. Wiremock-based unit tests cover happy path + key error modes.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::common::{get_api_key, map_http_error, map_reqwest_error, RateLimiter};
use crate::ai_router::{
    AiClient, AiRequest, AiResponse, Model, Provider, ProviderError, ProviderUsage, TaskKind,
};
use crate::keychain::KeyStore;

/// Default Kling AI base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.klingai.com";
/// Keychain service id under which the Kling key lives.
pub const KEYCHAIN_SERVICE: &str = "kling";
/// Default rate: Kling caps roughly at 5 rps on the video endpoints.
const DEFAULT_RATE_PER_SEC: usize = 5;
/// Default duration (seconds) for generated videos.
const DEFAULT_DURATION_SEC: u32 = 5;

/// Upper bound for task-status poll iterations. With exponential back-off
/// from 2s to 30s, this allows ~3 minutes for a video render before timing
/// out (Kling typical end-to-end is 30-90s for short clips).
const DEFAULT_POLL_MAX_ATTEMPTS: u32 = 20;
const DEFAULT_POLL_INITIAL_DELAY: Duration = Duration::from_secs(2);
const DEFAULT_POLL_MAX_DELAY: Duration = Duration::from_secs(30);

/// Endpoint suffix for polling task status. Kling's `text2video` and
/// `image2video` endpoints both surface task status under
/// `GET /v1/videos/text2video/{task_id}` (and likewise `image2video`).
/// We pass the endpoint path through `poll_task` to mirror the structure
/// used by `runway`.
const TEXT2VIDEO_PATH: &str = "/v1/videos/text2video";
const IMAGE2VIDEO_PATH: &str = "/v1/videos/image2video";

pub struct KlingClient {
    http: Client,
    base_url: String,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
    poll_max_attempts: u32,
    poll_initial_delay: Duration,
    poll_max_delay: Duration,
}

impl KlingClient {
    pub fn new(key_store: Arc<dyn KeyStore>) -> Self {
        Self::with_base_url(key_store, DEFAULT_BASE_URL.to_owned(), DEFAULT_RATE_PER_SEC)
    }

    pub fn with_base_url(
        key_store: Arc<dyn KeyStore>,
        base_url: String,
        rate_per_sec: usize,
    ) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("reqwest client builds");
        Self {
            http,
            base_url,
            key_store,
            rate: RateLimiter::new(rate_per_sec),
            poll_max_attempts: DEFAULT_POLL_MAX_ATTEMPTS,
            poll_initial_delay: DEFAULT_POLL_INITIAL_DELAY,
            poll_max_delay: DEFAULT_POLL_MAX_DELAY,
        }
    }

    /// Test-friendly variant that doesn't schedule a refill task.
    #[cfg(test)]
    pub fn for_test(key_store: Arc<dyn KeyStore>, base_url: String) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("reqwest client builds");
        Self {
            http,
            base_url,
            key_store,
            rate: RateLimiter::unlimited(),
            // Tests should not sit spinning for minutes; shrink every knob.
            poll_max_attempts: DEFAULT_POLL_MAX_ATTEMPTS,
            poll_initial_delay: Duration::from_millis(10),
            poll_max_delay: Duration::from_millis(100),
        }
    }

    /// Test-friendly variant that accepts a custom HTTP timeout. Used by the
    /// timeout-pillar wiremock tests to keep wall-clock cost low (single-digit
    /// ms instead of waiting on the default 5s timeout).
    #[cfg(test)]
    pub fn for_test_with_http_timeout(
        key_store: Arc<dyn KeyStore>,
        base_url: String,
        http_timeout: Duration,
    ) -> Self {
        let http = Client::builder()
            .timeout(http_timeout)
            .build()
            .expect("reqwest client builds");
        Self {
            http,
            base_url,
            key_store,
            rate: RateLimiter::unlimited(),
            poll_max_attempts: DEFAULT_POLL_MAX_ATTEMPTS,
            poll_initial_delay: Duration::from_millis(10),
            poll_max_delay: Duration::from_millis(100),
        }
    }

    /// Like [`Self::for_test`], but with a custom cap on poll attempts. Used
    /// by the timeout-specific test to guarantee fast failure when the task
    /// never reaches a terminal state.
    #[cfg(test)]
    pub fn for_test_with_poll_budget(
        key_store: Arc<dyn KeyStore>,
        base_url: String,
        max_attempts: u32,
    ) -> Self {
        let mut c = Self::for_test(key_store, base_url);
        c.poll_max_attempts = max_attempts;
        c
    }

    fn model_slug(model: Model) -> Option<&'static str> {
        match model {
            Model::Kling20 => Some("kling-2.0"),
            _ => None,
        }
    }

    async fn send_request(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        self.rate.acquire().await;
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;
        let slug = Self::model_slug(model)
            .ok_or_else(|| ProviderError::Permanent(format!("unsupported model {model:?}")))?;

        let duration = request
            .payload
            .get("duration")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(DEFAULT_DURATION_SEC);

        let (path, body) = match request.task {
            TaskKind::TextToVideo => {
                let body = json!({
                    "model_name": slug,
                    "prompt": request.prompt,
                    "duration": duration,
                });
                (TEXT2VIDEO_PATH, body)
            }
            TaskKind::ImageToVideo => {
                let image_url = request
                    .payload
                    .get("image_url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ProviderError::Permanent("kling image2video: image_url required".into())
                    })?;
                let body = json!({
                    "model_name": slug,
                    "prompt": request.prompt,
                    "image_url": image_url,
                    "duration": duration,
                });
                (IMAGE2VIDEO_PATH, body)
            }
            // Exhaustive arm — adding a new TaskKind without considering Kling
            // forces a compile error rather than silently routing to
            // `Permanent("unsupported")`.
            TaskKind::TextGeneration
            | TaskKind::ImageGeneration
            | TaskKind::ImageEdit
            | TaskKind::ImageAnalysis
            | TaskKind::Inpaint
            | TaskKind::Upscale
            | TaskKind::Logo
            | TaskKind::VideoMontage
            | TaskKind::Text3D
            | TaskKind::Image3D
            | TaskKind::DepthMap => {
                return Err(ProviderError::Permanent(format!(
                    "kling: unsupported task {:?}",
                    request.task
                )));
            }
        };

        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {key}"))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(map_reqwest_error)?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(map_http_error(status, &text));
        }

        let parsed: KlingVideoResponse = resp.json().await.map_err(map_reqwest_error)?;
        let task_id = parsed.task_id;
        if task_id.is_empty() {
            return Err(ProviderError::Permanent(
                "kling submit: missing task_id".into(),
            ));
        }

        // Poll for terminal status. Kling's task statuses are SUCCEED / FAILED /
        // SUBMITTED / PROCESSING (and others); we treat SUCCEED as completion.
        let final_body = self.poll_task(&task_id, path).await?;

        let video_url = final_body
            .get("video_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProviderError::Permanent("kling: SUCCEED returned without video_url".into())
            })?;

        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({
                "job_id": task_id,
                "status": "succeeded",
                "video_url": video_url,
            }),
            cost_cents: None,
            cached: false,
        })
    }

    /// Poll Kling for the task's terminal status. Mirrors
    /// [`super::runway::RunwayClient::poll_task`]. Accepts upper- and
    /// lower-case status strings; `SUCCEED` → return final body;
    /// `FAILED` → [`ProviderError::Permanent`]; exhausted attempts →
    /// [`ProviderError::Timeout`] (transient — router may fall back).
    async fn poll_task(&self, task_id: &str, endpoint_path: &str) -> Result<Value, ProviderError> {
        let url = format!("{}{}/{}", self.base_url, endpoint_path, task_id);
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;
        let mut delay = self.poll_initial_delay;

        for _ in 0..self.poll_max_attempts {
            // Rate-limit every poll so concurrent video jobs can't burst GETs
            // past Kling's per-second cap. `RateLimiter::unlimited()` (used by
            // `for_test*`) has no refill, so this is a zero-cost no-op in tests.
            self.rate.acquire().await;
            let resp = self
                .http
                .get(&url)
                .header("Authorization", format!("Bearer {key}"))
                .send()
                .await
                .map_err(map_reqwest_error)?;

            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(map_http_error(status, &text));
            }
            let body: Value = resp.json().await.map_err(map_reqwest_error)?;

            // Kling has historically shipped both UPPER and lower case status
            // tokens (`SUCCEED`, `succeed`) — accept either. `task_status` is
            // the documented field; `status` is accepted as a fallback.
            let s = body
                .get("task_status")
                .or_else(|| body.get("status"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_ascii_uppercase());
            match s.as_deref() {
                Some("SUCCEED" | "SUCCEEDED" | "COMPLETED") => return Ok(body),
                Some("FAILED" | "FAIL" | "ERROR") => {
                    let msg = body
                        .get("error")
                        .and_then(|v| v.as_str())
                        .or_else(|| body.get("message").and_then(|v| v.as_str()))
                        .or_else(|| body.get("task_status_msg").and_then(|v| v.as_str()))
                        .unwrap_or("kling task failed");
                    return Err(ProviderError::Permanent(msg.to_string()));
                }
                // SUBMITTED / PROCESSING / unknown — keep polling.
                _ => {
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(self.poll_max_delay);
                }
            }
        }

        Err(ProviderError::Timeout)
    }
}

/// Kling's submission response — shared across the `text2video` and
/// `image2video` endpoints. Both return the same envelope:
/// `{ task_id, task_status }`. `video_url` only appears on the polled
/// task-status endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KlingVideoResponse {
    #[serde(default)]
    task_id: String,
    #[serde(default, alias = "status")]
    #[allow(dead_code)]
    task_status: String,
}

#[async_trait]
impl AiClient for KlingClient {
    fn provider(&self) -> Provider {
        Provider::Kling
    }

    fn supports(&self, model: Model) -> bool {
        Self::model_slug(model).is_some()
    }

    async fn execute(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        self.send_request(model, request).await
    }

    async fn health_check(&self) -> bool {
        // Kling health is simply "do we have a key to call with?"
        self.key_store.get(KEYCHAIN_SERVICE).is_ok()
    }

    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
        // Kling doesn't expose a stable usage endpoint — tracked via
        // per-response task metadata.
        Ok(ProviderUsage {
            notes: Some("Kling quota tracked via task polling".into()),
            ..ProviderUsage::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_router::{Complexity, Priority, TaskKind};
    use crate::keychain::InMemoryStore;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn key_store_with_key() -> Arc<dyn KeyStore> {
        let s = InMemoryStore::new();
        s.store(KEYCHAIN_SERVICE, "kling-test-key").unwrap();
        Arc::new(s)
    }

    fn request(prompt: &str) -> AiRequest {
        AiRequest {
            id: "kling-r1".into(),
            task: TaskKind::TextToVideo,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: prompt.into(),
            payload: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn happy_path_returns_task_job() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(TEXT2VIDEO_PATH))
            .and(header("Authorization", "Bearer kling-test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "task_id": "task-42",
                "task_status": "submitted"
            })))
            .expect(1)
            .mount(&server)
            .await;

        // Polling now happens inline; immediately return SUCCEED with a
        // video_url.
        Mock::given(method("GET"))
            .and(path(format!("{}/task-42", TEXT2VIDEO_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "task_id": "task-42",
                "task_status": "succeed",
                "video_url": "https://fake.kling/v.mp4"
            })))
            .mount(&server)
            .await;

        let client = KlingClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(Model::Kling20, &request("a cat flying"))
            .await
            .unwrap();
        assert_eq!(resp.model, Model::Kling20);
        assert_eq!(
            resp.output.get("job_id").unwrap().as_str().unwrap(),
            "task-42"
        );
        assert_eq!(
            resp.output.get("status").unwrap().as_str().unwrap(),
            "succeeded"
        );
        assert_eq!(
            resp.output.get("video_url").unwrap().as_str().unwrap(),
            "https://fake.kling/v.mp4"
        );
    }

    #[tokio::test]
    async fn missing_key_yields_auth_error() {
        let server = MockServer::start().await;
        let empty = Arc::new(InMemoryStore::new()) as Arc<dyn KeyStore>;
        let client = KlingClient::for_test(empty, server.uri());
        let err = client
            .execute(Model::Kling20, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn server_500_is_transient() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(TEXT2VIDEO_PATH))
            .respond_with(ResponseTemplate::new(500).set_body_string("upstream"))
            .mount(&server)
            .await;
        let client = KlingClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::Kling20, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Transient(_)));
    }

    #[tokio::test]
    async fn status_401_is_auth() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(TEXT2VIDEO_PATH))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid key"))
            .mount(&server)
            .await;
        let client = KlingClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::Kling20, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn unsupported_model_is_permanent() {
        let server = MockServer::start().await;
        let client = KlingClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::ClaudeSonnet, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn supports_reports_only_kling_models() {
        let client = KlingClient::for_test(key_store_with_key(), "http://localhost".to_string());
        assert!(client.supports(Model::Kling20));
        assert!(!client.supports(Model::ClaudeSonnet));
        assert!(!client.supports(Model::RunwayGen3));
        assert!(!client.supports(Model::HiggsfieldMulti));
        assert!(!client.supports(Model::FalFluxPro));
    }

    #[tokio::test]
    async fn get_usage_returns_default_for_kling() {
        let client = KlingClient::for_test(key_store_with_key(), "http://localhost".to_string());
        let usage = client.get_usage().await.unwrap();
        assert!(usage.notes.is_some());
    }

    #[tokio::test]
    async fn image_to_video_hits_image2video_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(IMAGE2VIDEO_PATH))
            .and(body_partial_json(json!({
                "image_url": "https://src/a.png",
                "prompt": "cinematic"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "task_id": "t1",
                "task_status": "submitted"
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("{}/t1", IMAGE2VIDEO_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "task_id": "t1",
                "task_status": "succeed",
                "video_url": "https://out/v.mp4"
            })))
            .mount(&server)
            .await;

        let client = KlingClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "kling-i2v-1".into(),
            task: TaskKind::ImageToVideo,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: "cinematic".into(),
            payload: json!({ "image_url": "https://src/a.png" }),
        };
        let resp = client.execute(Model::Kling20, &req).await.unwrap();
        assert_eq!(
            resp.output.get("video_url").and_then(|v| v.as_str()),
            Some("https://out/v.mp4")
        );
    }

    #[tokio::test]
    async fn image_to_video_requires_image_url() {
        let server = MockServer::start().await;
        let client = KlingClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "kling-i2v-2".into(),
            task: TaskKind::ImageToVideo,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: "x".into(),
            payload: serde_json::Value::Null,
        };
        let err = client.execute(Model::Kling20, &req).await.unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn unsupported_task_is_permanent() {
        let server = MockServer::start().await;
        let client = KlingClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "kling-bad".into(),
            task: TaskKind::ImageGeneration,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: "x".into(),
            payload: serde_json::Value::Null,
        };
        let err = client.execute(Model::Kling20, &req).await.unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }

    /// Response delay exceeding the reqwest HTTP timeout (50ms here) on the
    /// POST must surface as `ProviderError::Timeout` so the router can fall
    /// back to a different provider instead of hanging the user. The test
    /// intentionally delays submission so polling never begins — keeping the
    /// existing pillar wire-shape from FU #197.
    #[tokio::test]
    async fn response_delay_yields_timeout() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(TEXT2VIDEO_PATH))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({
                        "task_id": "task-late",
                        "task_status": "submitted"
                    }))
                    .set_delay(std::time::Duration::from_millis(100)),
            )
            .mount(&server)
            .await;

        let client = KlingClient::for_test_with_http_timeout(
            key_store_with_key(),
            server.uri(),
            Duration::from_millis(50),
        );
        let err = client
            .execute(Model::Kling20, &request("hang please"))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ProviderError::Timeout),
            "expected Timeout, got {err:?}"
        );
    }

    #[tokio::test]
    async fn kling_propagates_failed_status() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(TEXT2VIDEO_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "task_id": "k-fail-1",
                "task_status": "submitted"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("{}/k-fail-1", TEXT2VIDEO_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "task_id": "k-fail-1",
                "task_status": "failed",
                "task_status_msg": "content moderation rejected"
            })))
            .mount(&server)
            .await;

        let client = KlingClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::Kling20, &request("x"))
            .await
            .unwrap_err();
        let s = err.to_string();
        assert!(
            matches!(err, ProviderError::Permanent(_)),
            "expected Permanent, got {err:?}"
        );
        assert!(
            s.contains("moderation"),
            "error should bubble Kling message, got: {s}"
        );
    }

    #[tokio::test]
    async fn kling_times_out_after_max_attempts() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(TEXT2VIDEO_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "task_id": "k-timeout-1",
                "task_status": "submitted"
            })))
            .mount(&server)
            .await;

        // Always PROCESSING — polling should give up.
        Mock::given(method("GET"))
            .and(path(format!("{}/k-timeout-1", TEXT2VIDEO_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "task_status": "processing"
            })))
            .mount(&server)
            .await;

        let client = KlingClient::for_test_with_poll_budget(key_store_with_key(), server.uri(), 2);
        let err = client
            .execute(Model::Kling20, &request("x"))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ProviderError::Timeout),
            "expected Timeout, got {err:?}"
        );
    }
}
