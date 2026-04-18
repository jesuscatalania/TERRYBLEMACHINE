//! Higgsfield client — multi-model video generation aggregator.
//!
//! Follows the reference pattern established in [`super::claude`] and the
//! polling loop from [`super::meshy`]:
//! 1. `new(key_store)` + `with_base_url(..)` keychain-backed constructors.
//! 2. Impl of [`AiClient`](crate::ai_router::AiClient) dispatches the single
//!    `/api/v1/generate` endpoint through `send_request`, which POSTs then
//!    polls `GET /api/v1/generate/{id}` until Higgsfield reports a terminal
//!    state.
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

/// Default Higgsfield API base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.higgsfield.com";
/// Keychain service id under which the Higgsfield key lives.
pub const KEYCHAIN_SERVICE: &str = "higgsfield";
/// Default rate: Higgsfield paid plans allow ~5 rps.
const DEFAULT_RATE_PER_SEC: usize = 5;
/// Sub-provider routed through Higgsfield's aggregator by default.
const DEFAULT_SUB_PROVIDER: &str = "higgsfield";

const GENERATE_PATH: &str = "/api/v1/generate";

/// Upper bound for task-status poll iterations. With exponential back-off from
/// 2s to 15s this sums to roughly 6 minutes of waiting before we surrender
/// and bubble a `Timeout` to the router (which may fall back). Matches
/// Higgsfield's per-clip generation window of several minutes.
const DEFAULT_POLL_MAX_ATTEMPTS: u32 = 30;
const DEFAULT_POLL_INITIAL_DELAY: Duration = Duration::from_secs(2);
const DEFAULT_POLL_MAX_DELAY: Duration = Duration::from_secs(15);

pub struct HiggsfieldClient {
    http: Client,
    base_url: String,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
    poll_max_attempts: u32,
    poll_initial_delay: Duration,
    poll_max_delay: Duration,
}

impl HiggsfieldClient {
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
            Model::HiggsfieldMulti => Some("higgsfield-multi"),
            _ => None,
        }
    }

    async fn send_request(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        let slug = Self::model_slug(model)
            .ok_or_else(|| ProviderError::Permanent(format!("unsupported model {model:?}")))?;

        let sub_provider = request
            .payload
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_SUB_PROVIDER);

        let body = json!({
            "model": slug,
            "prompt": request.prompt,
            "provider": sub_provider,
        });

        let task_id = self.start_task(GENERATE_PATH, &body).await?;
        let final_body = self.poll_task(&task_id, GENERATE_PATH).await?;

        // Higgsfield's terminal body carries the rendered clip under
        // `video.url` (aggregator-normalised) — fall back to `output.url` or a
        // bare `video_url` string for sub-providers that pass the raw
        // response through.
        let video_url = extract_video_url(&final_body).ok_or_else(|| {
            ProviderError::Permanent(
                "higgsfield: missing video.url / output.url on succeeded task".into(),
            )
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

    /// POST the body to `endpoint_path` and extract the job id. Higgsfield's
    /// response envelope is `{ id, state }` — we keep the `id` and hand it
    /// off to [`Self::poll_task`].
    async fn start_task(&self, endpoint_path: &str, body: &Value) -> Result<String, ProviderError> {
        self.rate.acquire().await;
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;

        let url = format!("{}{}", self.base_url, endpoint_path);
        let resp = self
            .http
            .post(&url)
            .header("x-api-key", key)
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

        let parsed: GenerateResponse = resp.json().await.map_err(map_reqwest_error)?;
        if parsed.id.is_empty() {
            return Err(ProviderError::Permanent(
                "higgsfield start: missing `id` job id".into(),
            ));
        }
        Ok(parsed.id)
    }

    /// Poll `GET {endpoint_path}/{task_id}` until Higgsfield reports a
    /// terminal state. Returns the final JSON body on `succeeded`; maps
    /// `failed` to [`ProviderError::Permanent`] (the content was rejected —
    /// no point retrying) and exhausted attempts to
    /// [`ProviderError::Timeout`] (transient — router may fall back).
    async fn poll_task(&self, task_id: &str, endpoint_path: &str) -> Result<Value, ProviderError> {
        let url = format!("{}{}/{}", self.base_url, endpoint_path, task_id);
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;
        let mut delay = self.poll_initial_delay;

        for _ in 0..self.poll_max_attempts {
            // Rate-limit every poll so concurrent video jobs can't burst GETs
            // past Higgsfield's per-second cap. `RateLimiter::unlimited()`
            // (used by `for_test*`) has `usize::MAX >> 4` permits and no
            // refill, so this is a zero-cost no-op in tests.
            self.rate.acquire().await;
            let resp = self
                .http
                .get(&url)
                .header("x-api-key", &key)
                .send()
                .await
                .map_err(map_reqwest_error)?;

            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(map_http_error(status, &text));
            }
            let body: Value = resp.json().await.map_err(map_reqwest_error)?;

            // Higgsfield uses `state` — some sub-providers mirror it as
            // `status`. Accept both and normalise to lower-case.
            let s = body
                .get("state")
                .or_else(|| body.get("status"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_ascii_lowercase());
            match s.as_deref() {
                Some("succeeded") | Some("success") | Some("completed") => return Ok(body),
                Some("failed") | Some("error") => {
                    let msg = body
                        .get("error")
                        .and_then(|v| v.as_str())
                        .or_else(|| {
                            body.get("error")
                                .and_then(|v| v.get("message"))
                                .and_then(|v| v.as_str())
                        })
                        .or_else(|| body.get("message").and_then(|v| v.as_str()))
                        .unwrap_or("higgsfield task failed");
                    return Err(ProviderError::Permanent(msg.to_string()));
                }
                // queued / processing / unknown — keep polling.
                _ => {
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(self.poll_max_delay);
                }
            }
        }

        Err(ProviderError::Timeout)
    }
}

/// Extract a playable video URL from a Higgsfield terminal body. Tries
/// `video.url`, then `output.url`, then a bare `video_url` string.
fn extract_video_url(body: &Value) -> Option<&str> {
    body.get("video")
        .and_then(|v| v.get("url"))
        .and_then(|v| v.as_str())
        .or_else(|| {
            body.get("output")
                .and_then(|v| v.get("url"))
                .and_then(|v| v.as_str())
        })
        .or_else(|| body.get("video_url").and_then(|v| v.as_str()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GenerateResponse {
    #[serde(default)]
    id: String,
    #[serde(default)]
    state: String,
}

#[async_trait]
impl AiClient for HiggsfieldClient {
    fn provider(&self) -> Provider {
        Provider::Higgsfield
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
        // Higgsfield has no dedicated ping endpoint — a provisioned key is
        // the closest proxy for "client is usable".
        self.key_store.get(KEYCHAIN_SERVICE).is_ok()
    }

    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
        // Higgsfield exposes credits via dashboard only.
        Ok(ProviderUsage {
            notes: Some("Higgsfield credits visible on dashboard".into()),
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
        s.store(KEYCHAIN_SERVICE, "higgs-test-key").unwrap();
        Arc::new(s)
    }

    fn request(prompt: &str) -> AiRequest {
        AiRequest {
            id: "higgs-r1".into(),
            task: TaskKind::TextToVideo,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: prompt.into(),
            payload: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn happy_path_returns_video_url() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(GENERATE_PATH))
            .and(header("x-api-key", "higgs-test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "job-abc",
                "state": "queued"
            })))
            .expect(1)
            .mount(&server)
            .await;

        // Polling happens inline; immediately return succeeded with a URL.
        Mock::given(method("GET"))
            .and(path(format!("{}/job-abc", GENERATE_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "job-abc",
                "state": "succeeded",
                "video": { "url": "https://fake.higgs/v.mp4" }
            })))
            .mount(&server)
            .await;

        let client = HiggsfieldClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(Model::HiggsfieldMulti, &request("cinematic shot"))
            .await
            .unwrap();
        assert_eq!(resp.model, Model::HiggsfieldMulti);
        assert_eq!(
            resp.output.get("job_id").unwrap().as_str().unwrap(),
            "job-abc"
        );
        assert_eq!(
            resp.output.get("status").unwrap().as_str().unwrap(),
            "succeeded"
        );
        assert_eq!(
            resp.output.get("video_url").unwrap().as_str().unwrap(),
            "https://fake.higgs/v.mp4"
        );
    }

    #[tokio::test]
    async fn higgsfield_text_to_video_polls_until_succeeded() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(GENERATE_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "h-poll-1",
                "state": "queued"
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("{}/h-poll-1", GENERATE_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "h-poll-1",
                "state": "processing"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("{}/h-poll-1", GENERATE_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "h-poll-1",
                "state": "succeeded",
                "video": { "url": "https://fake.higgs/poll-ok.mp4" }
            })))
            .mount(&server)
            .await;

        let client = HiggsfieldClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(Model::HiggsfieldMulti, &request("dolly-in on subject"))
            .await
            .expect("higgsfield polling succeeds");
        assert_eq!(
            resp.output.get("video_url").and_then(|v| v.as_str()),
            Some("https://fake.higgs/poll-ok.mp4")
        );
        assert_eq!(
            resp.output.get("status").and_then(|v| v.as_str()),
            Some("succeeded")
        );
        assert_eq!(
            resp.output.get("job_id").and_then(|v| v.as_str()),
            Some("h-poll-1")
        );
    }

    #[tokio::test]
    async fn higgsfield_propagates_failed_status() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(GENERATE_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "h-fail-1",
                "state": "queued"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("{}/h-fail-1", GENERATE_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "h-fail-1",
                "state": "failed",
                "error": "upstream sub-provider unavailable"
            })))
            .mount(&server)
            .await;

        let client = HiggsfieldClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::HiggsfieldMulti, &request("x"))
            .await
            .unwrap_err();
        let s = err.to_string();
        assert!(
            matches!(err, ProviderError::Permanent(_)),
            "expected Permanent, got {err:?}"
        );
        assert!(
            s.contains("sub-provider"),
            "error should bubble Higgsfield message, got: {s}"
        );
    }

    #[tokio::test]
    async fn higgsfield_times_out_after_max_attempts() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(GENERATE_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "h-timeout-1",
                "state": "queued"
            })))
            .mount(&server)
            .await;

        // Always processing — polling should give up.
        Mock::given(method("GET"))
            .and(path(format!("{}/h-timeout-1", GENERATE_PATH)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "state": "processing"
            })))
            .mount(&server)
            .await;

        let client =
            HiggsfieldClient::for_test_with_poll_budget(key_store_with_key(), server.uri(), 2);
        let err = client
            .execute(Model::HiggsfieldMulti, &request("x"))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ProviderError::Timeout),
            "expected Timeout, got {err:?}"
        );
    }

    #[tokio::test]
    async fn missing_key_yields_auth_error() {
        let server = MockServer::start().await;
        let empty = Arc::new(InMemoryStore::new()) as Arc<dyn KeyStore>;
        let client = HiggsfieldClient::for_test(empty, server.uri());
        let err = client
            .execute(Model::HiggsfieldMulti, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn server_500_is_transient() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(GENERATE_PATH))
            .respond_with(ResponseTemplate::new(500).set_body_string("upstream"))
            .mount(&server)
            .await;
        let client = HiggsfieldClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::HiggsfieldMulti, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Transient(_)));
    }

    #[tokio::test]
    async fn status_401_is_auth() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(GENERATE_PATH))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid key"))
            .mount(&server)
            .await;
        let client = HiggsfieldClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::HiggsfieldMulti, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn unsupported_model_is_permanent() {
        let server = MockServer::start().await;
        let client = HiggsfieldClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::Kling20, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn supports_reports_only_higgsfield_models() {
        let client =
            HiggsfieldClient::for_test(key_store_with_key(), "http://localhost".to_string());
        assert!(client.supports(Model::HiggsfieldMulti));
        assert!(!client.supports(Model::ClaudeSonnet));
        assert!(!client.supports(Model::Kling20));
        assert!(!client.supports(Model::RunwayGen3));
        assert!(!client.supports(Model::FalFluxPro));
    }

    #[tokio::test]
    async fn get_usage_returns_default_for_higgsfield() {
        let client =
            HiggsfieldClient::for_test(key_store_with_key(), "http://localhost".to_string());
        let usage = client.get_usage().await.unwrap();
        assert!(usage.notes.is_some());
    }
}
