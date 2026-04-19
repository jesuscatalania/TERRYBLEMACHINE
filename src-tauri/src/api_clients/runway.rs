//! Runway Gen-3 client — image-to-video generation.
//!
//! Follows the reference pattern established in [`super::claude`] and the
//! polling loop from [`super::meshy`]:
//! 1. `new(key_store)` + `with_base_url(..)` keychain-backed constructors.
//! 2. Impl of [`AiClient`](crate::ai_router::AiClient) dispatches the single
//!    image-to-video endpoint through `send_request` which POSTs then polls
//!    `GET /v1/image_to_video/{id}` until Runway reports a terminal status.
//! 3. Wiremock-based unit tests cover happy path, polling transitions,
//!    failure propagation + key error modes.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::common::{get_api_key, map_http_error, map_reqwest_error, RateLimiter};
use crate::ai_router::{
    AiClient, AiRequest, AiResponse, Model, Provider, ProviderError, ProviderUsage,
};
use crate::keychain::KeyStore;

/// Default Runway API base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.dev.runwayml.com";
/// Keychain service id under which the Runway key lives.
pub const KEYCHAIN_SERVICE: &str = "runway";
/// Default rate limit — Runway Pro allows ~5 rps on the generation endpoints.
const DEFAULT_RATE_PER_SEC: usize = 5;

const IMAGE_TO_VIDEO_PATH: &str = "/v1/image_to_video";

/// Upper bound for task-status poll iterations. With exponential back-off from
/// 2s to 15s this sums to roughly 6 minutes of waiting before we surrender and
/// bubble a `Timeout` to the router (which may fall back). Matches Runway's
/// documented per-clip generation window of several minutes.
const DEFAULT_POLL_MAX_ATTEMPTS: u32 = 30;
const DEFAULT_POLL_INITIAL_DELAY: Duration = Duration::from_secs(2);
const DEFAULT_POLL_MAX_DELAY: Duration = Duration::from_secs(15);

pub struct RunwayClient {
    http: Client,
    base_url: String,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
    poll_max_attempts: u32,
    poll_initial_delay: Duration,
    poll_max_delay: Duration,
}

impl RunwayClient {
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
            Model::RunwayGen3 => Some("gen3"),
            _ => None,
        }
    }

    async fn send_request(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        // Validate the model mapping even though the slug isn't sent over the
        // wire — Runway keys the model via the endpoint path.
        let _slug = Self::model_slug(model)
            .ok_or_else(|| ProviderError::Permanent(format!("unsupported model {model:?}")))?;

        let seed = request
            .payload
            .get("seed")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let mut body = json!({
            "promptText": request.prompt,
            "seed": seed,
        });

        // Forward Motion Brush payload if the caller supplied one. Runway's
        // image_to_video endpoint accepts a top-level `motion_brush` object
        // (e.g. `{ "strokes": [...] }`). A scalar / array / null shape is a
        // caller bug — fail fast with a permanent error rather than passing
        // garbage to Runway and waiting for a 4xx response.
        if let Some(motion_brush) = request.payload.get("motion_brush") {
            if !motion_brush.is_object() {
                return Err(ProviderError::Permanent(
                    "runway: motion_brush must be an object".into(),
                ));
            }
            if let Some(obj) = body.as_object_mut() {
                obj.insert("motion_brush".into(), motion_brush.clone());
            }
        }

        let task_id = self.start_task(IMAGE_TO_VIDEO_PATH, &body).await?;
        let final_body = self.poll_task(&task_id, IMAGE_TO_VIDEO_PATH).await?;

        // Runway's terminal body is `{ status: "SUCCEEDED", output: ["…url…"] }`.
        // `output` is an array — take the first entry.
        let video_url = final_body
            .get("output")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProviderError::Permanent("runway: missing output[0] on succeeded task".into())
            })?;

        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({
                "job_id": task_id,
                "video_url": video_url,
                "status": "succeeded",
            }),
            cost_cents: None,
            cached: false,
        })
    }

    /// POST the body to `endpoint_path` and extract the task id. Runway's
    /// response envelope is `{ id, status }` — we keep the `id` and hand it
    /// off to [`Self::poll_task`].
    async fn start_task(&self, endpoint_path: &str, body: &Value) -> Result<String, ProviderError> {
        self.rate.acquire().await;
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;

        let url = format!("{}{}", self.base_url, endpoint_path);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {key}"))
            .header("content-type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(map_reqwest_error)?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(map_http_error(status, &text));
        }

        let parsed: ImageToVideoResponse = resp.json().await.map_err(map_reqwest_error)?;
        if parsed.id.is_empty() {
            return Err(ProviderError::Permanent(
                "runway start: missing `id` task id".into(),
            ));
        }
        Ok(parsed.id)
    }

    /// Poll `GET {endpoint_path}/{task_id}` until Runway reports a terminal
    /// status. Returns the final JSON body on `SUCCEEDED`; maps `FAILED` to
    /// [`ProviderError::Permanent`] (the content was rejected — no point
    /// retrying) and exhausted attempts to
    /// [`ProviderError::JobAlreadySubmitted`] (NOT retriable — Runway is
    /// already running the job and a retry would create a duplicate that
    /// gets billed).
    async fn poll_task(&self, task_id: &str, endpoint_path: &str) -> Result<Value, ProviderError> {
        let url = format!("{}{}/{}", self.base_url, endpoint_path, task_id);
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;
        let mut delay = self.poll_initial_delay;

        for _ in 0..self.poll_max_attempts {
            // Rate-limit every poll so concurrent video jobs can't burst GETs
            // past Runway's per-second cap. `RateLimiter::unlimited()` (used by
            // `for_test*`) has `usize::MAX >> 4` permits and no refill, so this
            // is a zero-cost no-op in tests.
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

            // Runway has historically shipped both UPPER and lower case status
            // tokens — accept either.
            let s = body
                .get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_ascii_uppercase());
            match s.as_deref() {
                Some("SUCCEEDED") => return Ok(body),
                Some("FAILED") => {
                    let msg = body
                        .get("error")
                        .and_then(|v| v.as_str())
                        .or_else(|| body.get("failure").and_then(|v| v.as_str()))
                        .or_else(|| body.get("message").and_then(|v| v.as_str()))
                        .unwrap_or("runway task failed");
                    return Err(ProviderError::Permanent(msg.to_string()));
                }
                // PENDING / RUNNING / THROTTLED / unknown — keep polling.
                _ => {
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(self.poll_max_delay);
                }
            }
        }

        Err(ProviderError::JobAlreadySubmitted(format!(
            "runway task {task_id} did not reach a terminal status within {} poll attempts",
            self.poll_max_attempts
        )))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ImageToVideoResponse {
    #[serde(default)]
    id: String,
    #[serde(default)]
    status: String,
}

#[async_trait]
impl AiClient for RunwayClient {
    fn provider(&self) -> Provider {
        Provider::Runway
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
        // Runway has no public ping endpoint on the free dev API, so we
        // restrict health to "is a key provisioned?".
        self.key_store.get(KEYCHAIN_SERVICE).is_ok()
    }

    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
        // Runway exposes credits via response headers on each call; we
        // surface a hint rather than a live usage snapshot.
        Ok(ProviderUsage {
            notes: Some("Runway credits reported per-response headers".into()),
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
        s.store(KEYCHAIN_SERVICE, "runway-test-key").unwrap();
        Arc::new(s)
    }

    fn request(prompt: &str) -> AiRequest {
        AiRequest {
            id: "runway-r1".into(),
            task: TaskKind::ImageToVideo,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: prompt.into(),
            payload: serde_json::Value::Null,
            model_override: None,
        }
    }

    #[tokio::test]
    async fn happy_path_returns_video_url() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(IMAGE_TO_VIDEO_PATH))
            .and(header("Authorization", "Bearer runway-test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "gen-77",
                "status": "PENDING"
            })))
            .expect(1)
            .mount(&server)
            .await;

        // Polling now happens inline; immediately return SUCCEEDED with an
        // output URL.
        Mock::given(method("GET"))
            .and(path(format!("{}/gen-77", IMAGE_TO_VIDEO_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "gen-77",
                "status": "SUCCEEDED",
                "output": ["https://fake.runway/v.mp4"]
            })))
            .mount(&server)
            .await;

        let client = RunwayClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(Model::RunwayGen3, &request("a drone shot"))
            .await
            .unwrap();
        assert_eq!(resp.model, Model::RunwayGen3);
        assert_eq!(
            resp.output.get("job_id").unwrap().as_str().unwrap(),
            "gen-77"
        );
        assert_eq!(
            resp.output.get("status").unwrap().as_str().unwrap(),
            "succeeded"
        );
        assert_eq!(
            resp.output.get("video_url").unwrap().as_str().unwrap(),
            "https://fake.runway/v.mp4"
        );
    }

    #[tokio::test]
    async fn runway_text_to_video_polls_until_succeeded() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(IMAGE_TO_VIDEO_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "r-poll-1",
                "status": "PENDING"
            })))
            .expect(1)
            .mount(&server)
            .await;

        // First GET responds IN_PROGRESS (budget of 1), then the second mount
        // takes over and returns SUCCEEDED. wiremock processes mocks FIFO and
        // retires mocks whose call budget is exhausted.
        Mock::given(method("GET"))
            .and(path(format!("{}/r-poll-1", IMAGE_TO_VIDEO_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "r-poll-1",
                "status": "IN_PROGRESS"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("{}/r-poll-1", IMAGE_TO_VIDEO_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "r-poll-1",
                "status": "SUCCEEDED",
                "output": ["https://fake.runway/poll-ok.mp4"]
            })))
            .mount(&server)
            .await;

        let client = RunwayClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(Model::RunwayGen3, &request("a pan shot"))
            .await
            .expect("runway polling succeeds");
        assert_eq!(
            resp.output.get("video_url").and_then(|v| v.as_str()),
            Some("https://fake.runway/poll-ok.mp4")
        );
        assert_eq!(
            resp.output.get("status").and_then(|v| v.as_str()),
            Some("succeeded")
        );
        assert_eq!(
            resp.output.get("job_id").and_then(|v| v.as_str()),
            Some("r-poll-1")
        );
    }

    #[tokio::test]
    async fn runway_propagates_failed_status() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(IMAGE_TO_VIDEO_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "r-fail-1",
                "status": "PENDING"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("{}/r-fail-1", IMAGE_TO_VIDEO_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "r-fail-1",
                "status": "FAILED",
                "error": "content moderation rejected"
            })))
            .mount(&server)
            .await;

        let client = RunwayClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::RunwayGen3, &request("x"))
            .await
            .unwrap_err();
        let s = err.to_string();
        assert!(
            matches!(err, ProviderError::Permanent(_)),
            "expected Permanent, got {err:?}"
        );
        assert!(
            s.contains("moderation"),
            "error should bubble Runway message, got: {s}"
        );
    }

    #[tokio::test]
    async fn runway_times_out_after_max_attempts() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(IMAGE_TO_VIDEO_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "r-timeout-1",
                "status": "PENDING"
            })))
            .mount(&server)
            .await;

        // Always IN_PROGRESS — polling should give up.
        Mock::given(method("GET"))
            .and(path(format!("{}/r-timeout-1", IMAGE_TO_VIDEO_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "IN_PROGRESS"
            })))
            .mount(&server)
            .await;

        let client = RunwayClient::for_test_with_poll_budget(key_store_with_key(), server.uri(), 2);
        let err = client
            .execute(Model::RunwayGen3, &request("x"))
            .await
            .unwrap_err();
        // Poll exhaustion must surface as JobAlreadySubmitted (NOT Timeout)
        // so the router doesn't re-POST and create a duplicate billable job.
        match err {
            ProviderError::JobAlreadySubmitted(msg) => {
                assert!(
                    msg.contains("r-timeout-1"),
                    "message should reference task_id, got: {msg}"
                );
            }
            other => panic!("expected JobAlreadySubmitted, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn missing_key_yields_auth_error() {
        let server = MockServer::start().await;
        let empty = Arc::new(InMemoryStore::new()) as Arc<dyn KeyStore>;
        let client = RunwayClient::for_test(empty, server.uri());
        let err = client
            .execute(Model::RunwayGen3, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn server_500_is_transient() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(IMAGE_TO_VIDEO_PATH))
            .respond_with(ResponseTemplate::new(500).set_body_string("upstream"))
            .mount(&server)
            .await;
        let client = RunwayClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::RunwayGen3, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Transient(_)));
    }

    #[tokio::test]
    async fn status_401_is_auth() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(IMAGE_TO_VIDEO_PATH))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid key"))
            .mount(&server)
            .await;
        let client = RunwayClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::RunwayGen3, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn unsupported_model_is_permanent() {
        let server = MockServer::start().await;
        let client = RunwayClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::Kling20, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn supports_reports_only_runway_models() {
        let client = RunwayClient::for_test(key_store_with_key(), "http://localhost".to_string());
        assert!(client.supports(Model::RunwayGen3));
        assert!(!client.supports(Model::ClaudeSonnet));
        assert!(!client.supports(Model::Kling20));
        assert!(!client.supports(Model::HiggsfieldMulti));
        assert!(!client.supports(Model::FalFluxPro));
    }

    #[tokio::test]
    async fn get_usage_returns_default_for_runway() {
        let client = RunwayClient::for_test(key_store_with_key(), "http://localhost".to_string());
        let usage = client.get_usage().await.unwrap();
        assert!(usage.notes.is_some());
    }

    #[tokio::test]
    async fn motion_brush_scalar_is_rejected() {
        let server = MockServer::start().await;
        let client = RunwayClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "runway-mb-bad".into(),
            task: TaskKind::ImageToVideo,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: "x".into(),
            // motion_brush as a scalar — must be rejected before hitting Runway.
            payload: json!({ "motion_brush": "not-an-object" }),
            model_override: None,
        };
        let err = client.execute(Model::RunwayGen3, &req).await.unwrap_err();
        match err {
            ProviderError::Permanent(msg) => {
                assert!(msg.contains("motion_brush"), "msg: {msg}");
            }
            other => panic!("expected Permanent, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn motion_brush_strokes_forwarded() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(IMAGE_TO_VIDEO_PATH))
            .and(body_partial_json(json!({
                "motion_brush": { "strokes": [{"x": 10, "y": 20, "dx": 5, "dy": 0}] }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "r1",
                "status": "PENDING"
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("{}/r1", IMAGE_TO_VIDEO_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "r1",
                "status": "SUCCEEDED",
                "output": ["https://out/v.mp4"]
            })))
            .mount(&server)
            .await;

        let client = RunwayClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "runway-mb-1".into(),
            task: TaskKind::ImageToVideo,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: "a panning shot".into(),
            payload: json!({
                "motion_brush": { "strokes": [{"x": 10, "y": 20, "dx": 5, "dy": 0}] }
            }),
            model_override: None,
        };
        let resp = client.execute(Model::RunwayGen3, &req).await.unwrap();
        assert_eq!(
            resp.output.get("job_id").and_then(|v| v.as_str()),
            Some("r1")
        );
        assert_eq!(
            resp.output.get("video_url").and_then(|v| v.as_str()),
            Some("https://out/v.mp4")
        );
    }
}
