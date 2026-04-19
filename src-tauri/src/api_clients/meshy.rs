//! Meshy Pro client (Text-to-3D + Image-to-3D).
//!
//! Follows the reference pattern established by [`super::claude`]:
//! keychain-backed constructor + wiremock unit tests. `execute` dispatches
//! on `Model` to either `send_text_3d` or `send_image_3d`, both of which
//! share the `start_task` + `poll_task` helpers that POST then poll
//! `GET {endpoint}/{task_id}` until Meshy reports `SUCCEEDED`.

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

/// Default Meshy base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.meshy.ai";
/// Keychain service id under which the Meshy key lives.
pub const KEYCHAIN_SERVICE: &str = "meshy";
/// Default rate limit: Meshy's public plan sits around 5 rps.
const DEFAULT_RATE_PER_SEC: usize = 5;

const TEXT_3D_PATH: &str = "/openapi/v2/text-to-3d";
const IMAGE_3D_PATH: &str = "/openapi/v2/image-to-3d";

/// Upper bound for task-status poll iterations. With exponential back-off from
/// 2s to 15s this sums to roughly 4 minutes of waiting before we surrender
/// and bubble a `Timeout` to the router (which may fall back). Matches
/// Meshy's documented "typical 2-5 min" mesh generation window — longer
/// jobs are better handled by a fresh request once the user returns.
const DEFAULT_POLL_MAX_ATTEMPTS: u32 = 20;
const DEFAULT_POLL_INITIAL_DELAY: Duration = Duration::from_secs(2);
const DEFAULT_POLL_MAX_DELAY: Duration = Duration::from_secs(15);

pub struct MeshyClient {
    http: Client,
    base_url: String,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
    poll_max_attempts: u32,
    poll_initial_delay: Duration,
    poll_max_delay: Duration,
}

impl MeshyClient {
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

    async fn send_text_3d(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        let body = json!({
            "mode": "preview",
            "prompt": request.prompt,
        });
        let task_id = self.start_task(TEXT_3D_PATH, &body).await?;
        let final_body = self.poll_task(&task_id, TEXT_3D_PATH).await?;
        let glb_url = final_body
            .get("model_urls")
            .and_then(|v| v.get("glb"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProviderError::Permanent("meshy text-to-3d: missing model_urls.glb".into())
            })?;

        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({
                "job_id": task_id,
                "glb_url": glb_url,
                "status": "succeeded",
            }),
            cost_cents: None,
            cached: false,
        })
    }

    async fn send_image_3d(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        let image_url = request
            .payload
            .get("image_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProviderError::Permanent("meshy image-to-3d: payload.image_url required".into())
            })?;
        let body = json!({ "image_url": image_url });

        let task_id = self.start_task(IMAGE_3D_PATH, &body).await?;
        let final_body = self.poll_task(&task_id, IMAGE_3D_PATH).await?;

        let glb_url = final_body
            .get("model_urls")
            .and_then(|v| v.get("glb"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProviderError::Permanent("meshy image-to-3d: missing model_urls.glb".into())
            })?;

        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({
                "job_id": task_id,
                "glb_url": glb_url,
                "status": "succeeded",
            }),
            cost_cents: None,
            cached: false,
        })
    }

    /// POST the body to `endpoint_path` and extract the `result` task id.
    /// Shared between `send_text_3d` and `send_image_3d`; both poll the
    /// returned task id via [`Self::poll_task`].
    async fn start_task(&self, endpoint_path: &str, body: &Value) -> Result<String, ProviderError> {
        self.rate.acquire().await;
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;

        let url = format!("{}{}", self.base_url, endpoint_path);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(key)
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

        let parsed: TaskResponse = resp.json().await.map_err(map_reqwest_error)?;
        if parsed.result.is_empty() {
            return Err(ProviderError::Permanent(
                "meshy start: missing `result` task id".into(),
            ));
        }
        Ok(parsed.result)
    }

    /// Poll `GET {endpoint_path}/{task_id}` until Meshy reports a terminal
    /// status. Returns the final JSON body on `SUCCEEDED`; maps `FAILED` to
    /// [`ProviderError::Permanent`] (no retry — the content was rejected) and
    /// exhausted attempts to [`ProviderError::JobAlreadySubmitted`] (NOT
    /// retriable — Meshy is already running the job and a retry would create
    /// a duplicate that gets billed).
    async fn poll_task(&self, task_id: &str, endpoint_path: &str) -> Result<Value, ProviderError> {
        let url = format!("{}{}/{}", self.base_url, endpoint_path, task_id);
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;
        let mut delay = self.poll_initial_delay;

        for _ in 0..self.poll_max_attempts {
            // Rate-limit every poll so concurrent mesh jobs can't burst GETs
            // past Meshy's per-second cap. `RateLimiter::unlimited()` (used by
            // `for_test*`) has `usize::MAX >> 4` permits and no refill, so
            // this is a zero-cost no-op in tests.
            self.rate.acquire().await;
            let resp = self
                .http
                .get(&url)
                .bearer_auth(&key)
                .send()
                .await
                .map_err(map_reqwest_error)?;

            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(map_http_error(status, &text));
            }
            let body: Value = resp.json().await.map_err(map_reqwest_error)?;

            match body.get("status").and_then(|v| v.as_str()) {
                Some("SUCCEEDED") => return Ok(body),
                Some("FAILED") => {
                    let msg = body
                        .get("task_error")
                        .and_then(|v| v.get("message"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("meshy task failed");
                    return Err(ProviderError::Permanent(msg.to_string()));
                }
                // PENDING / IN_PROGRESS / unknown — keep polling.
                _ => {
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(self.poll_max_delay);
                }
            }
        }

        Err(ProviderError::JobAlreadySubmitted(format!(
            "meshy task {task_id} did not reach a terminal status within {} poll attempts",
            self.poll_max_attempts
        )))
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct TaskResponse {
    #[serde(default)]
    result: String,
}

#[async_trait]
impl AiClient for MeshyClient {
    fn provider(&self) -> Provider {
        Provider::Meshy
    }

    fn supports(&self, model: Model) -> bool {
        matches!(model, Model::MeshyText3D | Model::MeshyImage3D)
    }

    async fn execute(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        match model {
            Model::MeshyText3D => self.send_text_3d(model, request).await,
            Model::MeshyImage3D => self.send_image_3d(model, request).await,
            _ => Err(ProviderError::Permanent(format!(
                "unsupported model {model:?}"
            ))),
        }
    }

    async fn health_check(&self) -> bool {
        self.key_store.get(KEYCHAIN_SERVICE).is_ok()
    }

    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
        Ok(ProviderUsage {
            notes: Some("Meshy usage tracked via dashboard".into()),
            ..ProviderUsage::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_router::{Complexity, Priority, TaskKind};
    use crate::keychain::InMemoryStore;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn key_store_with_key() -> Arc<dyn KeyStore> {
        let s = InMemoryStore::new();
        s.store(KEYCHAIN_SERVICE, "sk-test").unwrap();
        Arc::new(s)
    }

    fn text_request(prompt: &str) -> AiRequest {
        AiRequest {
            id: "r1".into(),
            task: TaskKind::Text3D,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: prompt.into(),
            payload: serde_json::Value::Null,
            model_override: None,
        }
    }

    fn image_request(image_url: &str) -> AiRequest {
        AiRequest {
            id: "r2".into(),
            task: TaskKind::Image3D,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: String::new(),
            payload: json!({ "image_url": image_url }),
            model_override: None,
        }
    }

    #[tokio::test]
    async fn happy_path_text_to_3d_returns_job_id() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(TEXT_3D_PATH))
            .and(header("authorization", "Bearer sk-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": "task-text-123"
            })))
            .expect(1)
            .mount(&server)
            .await;

        // Polling now happens inline; immediately return SUCCEEDED with a GLB.
        Mock::given(method("GET"))
            .and(path(format!("{}/task-text-123", TEXT_3D_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "task-text-123",
                "status": "SUCCEEDED",
                "model_urls": { "glb": "https://fake.meshy/model.glb" }
            })))
            .mount(&server)
            .await;

        let client = MeshyClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(Model::MeshyText3D, &text_request("a cute robot"))
            .await
            .unwrap();
        assert_eq!(resp.model, Model::MeshyText3D);
        assert_eq!(
            resp.output.get("job_id").unwrap().as_str().unwrap(),
            "task-text-123"
        );
        assert_eq!(
            resp.output.get("status").unwrap().as_str().unwrap(),
            "succeeded"
        );
        assert_eq!(
            resp.output.get("glb_url").unwrap().as_str().unwrap(),
            "https://fake.meshy/model.glb"
        );
    }

    #[tokio::test]
    async fn text_to_3d_polls_until_succeeded_then_returns_glb_url() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(TEXT_3D_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "result": "t1" })))
            .expect(1)
            .mount(&server)
            .await;

        // First GET responds IN_PROGRESS (budget of 1), then the second mount
        // takes over and returns SUCCEEDED. wiremock processes mocks FIFO and
        // retires mocks whose call budget is exhausted.
        Mock::given(method("GET"))
            .and(path(format!("{}/t1", TEXT_3D_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "t1",
                "status": "IN_PROGRESS",
                "progress": 25
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("{}/t1", TEXT_3D_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "t1",
                "status": "SUCCEEDED",
                "model_urls": { "glb": "https://fake.meshy/model.glb" }
            })))
            .mount(&server)
            .await;

        let client = MeshyClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(Model::MeshyText3D, &text_request("a coffee cup"))
            .await
            .expect("text 3d polling succeeds");

        assert_eq!(
            resp.output.get("glb_url").and_then(|v| v.as_str()),
            Some("https://fake.meshy/model.glb")
        );
        assert_eq!(
            resp.output.get("status").and_then(|v| v.as_str()),
            Some("succeeded")
        );
        assert_eq!(
            resp.output.get("job_id").and_then(|v| v.as_str()),
            Some("t1")
        );
    }

    #[tokio::test]
    async fn text_to_3d_propagates_failed_status() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(TEXT_3D_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "result": "t1" })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("{}/t1", TEXT_3D_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "FAILED",
                "task_error": { "message": "quota exceeded" }
            })))
            .mount(&server)
            .await;

        let client = MeshyClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::MeshyText3D, &text_request("x"))
            .await
            .unwrap_err();
        let s = err.to_string();
        assert!(
            matches!(err, ProviderError::Permanent(_)),
            "expected Permanent, got {err:?}"
        );
        assert!(
            s.contains("quota"),
            "error should bubble Meshy message, got: {s}"
        );
    }

    #[tokio::test]
    async fn text_to_3d_times_out_after_max_attempts() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(TEXT_3D_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "result": "t1" })))
            .mount(&server)
            .await;

        // Always IN_PROGRESS — polling should give up.
        Mock::given(method("GET"))
            .and(path(format!("{}/t1", TEXT_3D_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "IN_PROGRESS"
            })))
            .mount(&server)
            .await;

        let client = MeshyClient::for_test_with_poll_budget(key_store_with_key(), server.uri(), 2);
        let err = client
            .execute(Model::MeshyText3D, &text_request("x"))
            .await
            .unwrap_err();
        // Poll exhaustion must surface as JobAlreadySubmitted (NOT Timeout)
        // so the router doesn't re-POST and create a duplicate billable job.
        match err {
            ProviderError::JobAlreadySubmitted(msg) => {
                assert!(
                    msg.contains("t1"),
                    "message should reference task_id, got: {msg}"
                );
            }
            other => panic!("expected JobAlreadySubmitted, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn happy_path_image_to_3d_returns_job_id() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(IMAGE_3D_PATH))
            .and(header("authorization", "Bearer sk-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": "task-image-456"
            })))
            .expect(1)
            .mount(&server)
            .await;

        // Polling now happens inline; immediately return SUCCEEDED with a GLB.
        Mock::given(method("GET"))
            .and(path(format!("{}/task-image-456", IMAGE_3D_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "task-image-456",
                "status": "SUCCEEDED",
                "model_urls": { "glb": "https://fake.meshy/from-img.glb" }
            })))
            .mount(&server)
            .await;

        let client = MeshyClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(
                Model::MeshyImage3D,
                &image_request("https://example.com/img.png"),
            )
            .await
            .unwrap();
        assert_eq!(resp.model, Model::MeshyImage3D);
        assert_eq!(
            resp.output.get("job_id").unwrap().as_str().unwrap(),
            "task-image-456"
        );
        assert_eq!(
            resp.output.get("status").unwrap().as_str().unwrap(),
            "succeeded"
        );
        assert_eq!(
            resp.output.get("glb_url").unwrap().as_str().unwrap(),
            "https://fake.meshy/from-img.glb"
        );
    }

    #[tokio::test]
    async fn image_to_3d_polls_until_succeeded_then_returns_glb_url() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(IMAGE_3D_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "result": "i1" })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("{}/i1", IMAGE_3D_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "i1",
                "status": "IN_PROGRESS",
                "progress": 25
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("{}/i1", IMAGE_3D_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "i1",
                "status": "SUCCEEDED",
                "model_urls": { "glb": "https://fake.meshy/from-img.glb" }
            })))
            .mount(&server)
            .await;

        let client = MeshyClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(Model::MeshyImage3D, &image_request("https://src/a.png"))
            .await
            .expect("image 3d polling succeeds");

        assert_eq!(
            resp.output.get("glb_url").and_then(|v| v.as_str()),
            Some("https://fake.meshy/from-img.glb")
        );
        assert_eq!(
            resp.output.get("status").and_then(|v| v.as_str()),
            Some("succeeded")
        );
        assert_eq!(
            resp.output.get("job_id").and_then(|v| v.as_str()),
            Some("i1")
        );
    }

    #[tokio::test]
    async fn missing_key_yields_auth_error() {
        let server = MockServer::start().await;
        let empty = Arc::new(InMemoryStore::new()) as Arc<dyn KeyStore>;
        let client = MeshyClient::for_test(empty, server.uri());
        let err = client
            .execute(Model::MeshyText3D, &text_request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn server_500_is_transient() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(TEXT_3D_PATH))
            .respond_with(ResponseTemplate::new(500).set_body_string("upstream"))
            .mount(&server)
            .await;
        let client = MeshyClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::MeshyText3D, &text_request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Transient(_)));
    }

    #[tokio::test]
    async fn status_401_is_auth() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(TEXT_3D_PATH))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid key"))
            .mount(&server)
            .await;
        let client = MeshyClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::MeshyText3D, &text_request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn unsupported_model_is_permanent() {
        let server = MockServer::start().await;
        let client = MeshyClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::ClaudeSonnet, &text_request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn supports_reports_only_meshy_models() {
        let client = MeshyClient::for_test(key_store_with_key(), "http://localhost".to_string());
        assert!(client.supports(Model::MeshyText3D));
        assert!(client.supports(Model::MeshyImage3D));
        assert!(!client.supports(Model::ClaudeSonnet));
        assert!(!client.supports(Model::ShotstackMontage));
        assert!(!client.supports(Model::IdeogramV3));
    }
}
