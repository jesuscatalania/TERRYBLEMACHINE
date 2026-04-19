//! Shotstack JSON Timeline client (video montage, stage environment).
//!
//! Follows the reference pattern established by
//! [`super::claude`]: keychain-backed constructor + single `send_request`
//! pipeline + wiremock unit tests.
//!
//! In addition to the `AiClient::execute` entry point (which fits the router's
//! standard request shape and is used for ad-hoc montage tasks),
//! [`ShotstackClient::assemble_timeline`] + [`ShotstackClient::poll_render`]
//! expose the raw submit-then-poll dance used by the
//! [`shotstack_assembly`](crate::shotstack_assembly) pipeline. That pipeline
//! builds a full Shotstack timeline JSON and owns the MP4-download-to-cache
//! step; routing it through `AiClient::execute` would buy nothing (there's
//! no fallback for Shotstack-specific JSON) and force awkward payload gymnastics,
//! so we expose the two primitives directly.

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

/// Default Shotstack base URL (stage environment).
pub const DEFAULT_BASE_URL: &str = "https://api.shotstack.io";
/// Keychain service id under which the Shotstack key lives.
pub const KEYCHAIN_SERVICE: &str = "shotstack";
/// Default rate limit: Shotstack is an async render service — 5 rps is plenty.
const DEFAULT_RATE_PER_SEC: usize = 5;

/// Upper bound for render-status poll iterations. With exponential back-off
/// from 2s to 30s this sums to roughly 10 minutes of waiting before we
/// surrender — matches Shotstack's typical render window for short stage
/// clips. Longer jobs are better handled by a fresh poll once the user returns.
const DEFAULT_POLL_MAX_ATTEMPTS: u32 = 60;
const DEFAULT_POLL_INITIAL_DELAY: Duration = Duration::from_secs(2);
const DEFAULT_POLL_MAX_DELAY: Duration = Duration::from_secs(30);

/// Which Shotstack environment to hit. `Stage` is the sandbox — watermarked
/// output + a limited asset catalog, but free of charge and the default when
/// no prod key is provisioned. `Prod` switches to `/edit/v1/render`, billed
/// against the subscriber plan. Callers opt in via [`ShotstackClient::new_with_env`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShotstackEnv {
    #[default]
    Stage,
    Prod,
}

impl ShotstackEnv {
    /// Path prefix for the render endpoint. Used by both the submit POST
    /// and the status-poll GET (GET appends `/{render_id}`).
    pub fn render_path(self) -> &'static str {
        match self {
            Self::Stage => "/edit/stage/render",
            Self::Prod => "/edit/v1/render",
        }
    }
}

pub struct ShotstackClient {
    http: Client,
    base_url: String,
    env: ShotstackEnv,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
    poll_max_attempts: u32,
    poll_initial_delay: Duration,
    poll_max_delay: Duration,
}

impl ShotstackClient {
    pub fn new(key_store: Arc<dyn KeyStore>) -> Self {
        Self::with_base_url(
            key_store,
            DEFAULT_BASE_URL.to_owned(),
            DEFAULT_RATE_PER_SEC,
            ShotstackEnv::default(),
        )
    }

    /// Like [`Self::new`] but pins the environment explicitly. Use this when
    /// a prod key is wired up so renders hit `/edit/v1/render` (billed,
    /// un-watermarked) instead of the default stage sandbox.
    pub fn new_with_env(key_store: Arc<dyn KeyStore>, env: ShotstackEnv) -> Self {
        Self::with_base_url(
            key_store,
            DEFAULT_BASE_URL.to_owned(),
            DEFAULT_RATE_PER_SEC,
            env,
        )
    }

    pub fn with_base_url(
        key_store: Arc<dyn KeyStore>,
        base_url: String,
        rate_per_sec: usize,
        env: ShotstackEnv,
    ) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("reqwest client builds");
        Self {
            http,
            base_url,
            env,
            key_store,
            rate: RateLimiter::new(rate_per_sec),
            poll_max_attempts: DEFAULT_POLL_MAX_ATTEMPTS,
            poll_initial_delay: DEFAULT_POLL_INITIAL_DELAY,
            poll_max_delay: DEFAULT_POLL_MAX_DELAY,
        }
    }

    #[cfg(test)]
    pub fn for_test(key_store: Arc<dyn KeyStore>, base_url: String) -> Self {
        Self::for_test_with_env(key_store, base_url, ShotstackEnv::Stage)
    }

    #[cfg(test)]
    pub fn for_test_with_env(
        key_store: Arc<dyn KeyStore>,
        base_url: String,
        env: ShotstackEnv,
    ) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("reqwest client builds");
        Self {
            http,
            base_url,
            env,
            key_store,
            rate: RateLimiter::unlimited(),
            // Tests should not sit spinning for minutes; shrink every knob.
            poll_max_attempts: DEFAULT_POLL_MAX_ATTEMPTS,
            poll_initial_delay: Duration::from_millis(10),
            poll_max_delay: Duration::from_millis(100),
        }
    }

    /// Like [`Self::for_test`], but with a custom cap on poll attempts. Used
    /// by the timeout-specific test to guarantee fast failure when the render
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

    /// POST a pre-built Shotstack timeline body to the configured render
    /// endpoint (`/edit/stage/render` on Stage, `/edit/v1/render` on Prod)
    /// and return the render id. The caller is responsible for constructing
    /// a well-formed `{ timeline, output }` value — the
    /// [`shotstack_assembly`](crate::shotstack_assembly) pipeline does this
    /// from its own typed inputs.
    pub async fn assemble_timeline(&self, timeline_body: Value) -> Result<String, ProviderError> {
        self.rate.acquire().await;
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;

        let url = format!("{}{}", self.base_url, self.env.render_path());
        let resp = self
            .http
            .post(&url)
            .header("x-api-key", key)
            .header("content-type", "application/json")
            .json(&timeline_body)
            .send()
            .await
            .map_err(map_reqwest_error)?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(map_http_error(status, &text));
        }

        let parsed: RenderResponse = resp.json().await.map_err(map_reqwest_error)?;
        let id = parsed
            .response
            .as_ref()
            .map(|r| r.id.clone())
            .unwrap_or_default();
        if id.is_empty() {
            return Err(ProviderError::Permanent(
                "shotstack assemble: missing response.id".into(),
            ));
        }
        Ok(id)
    }

    /// Poll `GET {render_path}/{render_id}` until Shotstack reports a
    /// terminal status (`done` or `failed`). Returns the full JSON body on
    /// `done` so the caller can extract `response.url`; maps `failed` to
    /// [`ProviderError::Permanent`] with the server-supplied error message,
    /// and exhausted attempts to [`ProviderError::JobAlreadySubmitted`] (NOT
    /// retriable — Shotstack is already running the render and a retry would
    /// create a duplicate that gets billed; the user can chase the render id
    /// on the dashboard).
    pub async fn poll_render(&self, render_id: &str) -> Result<Value, ProviderError> {
        let url = format!("{}{}/{}", self.base_url, self.env.render_path(), render_id);
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;
        let mut delay = self.poll_initial_delay;

        for _ in 0..self.poll_max_attempts {
            // Rate-limit every poll so concurrent assembly jobs can't burst
            // GETs past Shotstack's per-second cap. `RateLimiter::unlimited()`
            // (used in tests) has `usize::MAX >> 4` permits and no refill, so
            // this is a zero-cost no-op in tests.
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

            match body.pointer("/response/status").and_then(|v| v.as_str()) {
                Some("done") => return Ok(body),
                Some("failed") => {
                    let msg = body
                        .pointer("/response/error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("shotstack render failed");
                    return Err(ProviderError::Permanent(msg.to_string()));
                }
                // queued / fetching / rendering / saving / unknown — keep polling.
                _ => {
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(self.poll_max_delay);
                }
            }
        }

        Err(ProviderError::JobAlreadySubmitted(format!(
            "shotstack render {render_id} did not reach a terminal status within {} poll attempts",
            self.poll_max_attempts
        )))
    }

    /// Build the render payload. Shotstack wants `{ timeline, output }`.
    /// `payload` on the request can override/extend these — for the happy-path
    /// test we fall back to a minimal sane default so the test doesn't have to
    /// stuff a whole timeline into the request body.
    ///
    /// Note: we deliberately do NOT forward `request.prompt` into the payload.
    /// The prompt is a user-facing description and the outgoing body should
    /// only carry Shotstack-intended fields; leaking it would both pollute
    /// render logs and risk exposing PII on the provider side. The canonical
    /// path for rich assembly input is [`Self::assemble_timeline`], called by
    /// the `shotstack_assembly` pipeline.
    fn build_body(request: &AiRequest) -> Value {
        let mut timeline = request
            .payload
            .get("timeline")
            .cloned()
            .unwrap_or_else(|| json!({ "tracks": [] }));
        if !timeline.is_object() {
            timeline = json!({ "tracks": [] });
        }
        let output = request
            .payload
            .get("output")
            .cloned()
            .unwrap_or_else(|| json!({ "format": "mp4", "resolution": "sd" }));

        let mut body = json!({
            "timeline": timeline,
            "output": output,
            "disk": "local",
        });
        if let Some(cb) = request.payload.get("callback") {
            body["callback"] = cb.clone();
        }
        body
    }

    async fn send_request(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        self.rate.acquire().await;
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;

        let body = Self::build_body(request);
        let url = format!("{}{}", self.base_url, self.env.render_path());

        let resp = self
            .http
            .post(&url)
            .header("x-api-key", key)
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

        let parsed: RenderResponse = resp.json().await.map_err(map_reqwest_error)?;
        let inner = parsed.response.unwrap_or_default();

        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({
                "job_id": inner.id,
                "status": "queued",
                "message": inner.message,
                "success": parsed.success,
            }),
            cost_cents: None,
            cached: false,
        })
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct RenderResponse {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    response: Option<RenderResponseInner>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct RenderResponseInner {
    #[serde(default)]
    id: String,
    #[serde(default)]
    message: Option<String>,
}

#[async_trait]
impl AiClient for ShotstackClient {
    fn provider(&self) -> Provider {
        Provider::Shotstack
    }

    fn supports(&self, model: Model) -> bool {
        matches!(model, Model::ShotstackMontage)
    }

    async fn execute(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        if !self.supports(model) {
            return Err(ProviderError::Permanent(format!(
                "unsupported model {model:?}"
            )));
        }
        self.send_request(model, request).await
    }

    async fn health_check(&self) -> bool {
        self.key_store.get(KEYCHAIN_SERVICE).is_ok()
    }

    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
        Ok(ProviderUsage {
            notes: Some("Shotstack usage tracked via dashboard".into()),
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

    fn request(prompt: &str) -> AiRequest {
        AiRequest {
            id: "r1".into(),
            task: TaskKind::VideoMontage,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: prompt.into(),
            payload: serde_json::Value::Null,
            model_override: None,
        }
    }

    #[tokio::test]
    async fn happy_path_returns_job_id() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/edit/stage/render"))
            .and(header("x-api-key", "sk-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "message": "Created",
                "response": {
                    "id": "render-abc-123",
                    "message": "Render queued"
                }
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = ShotstackClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(Model::ShotstackMontage, &request("assemble clip"))
            .await
            .unwrap();
        assert_eq!(resp.model, Model::ShotstackMontage);
        assert_eq!(
            resp.output.get("job_id").unwrap().as_str().unwrap(),
            "render-abc-123"
        );
        assert_eq!(
            resp.output.get("status").unwrap().as_str().unwrap(),
            "queued"
        );
    }

    #[tokio::test]
    async fn missing_key_yields_auth_error() {
        let server = MockServer::start().await;
        let empty = Arc::new(InMemoryStore::new()) as Arc<dyn KeyStore>;
        let client = ShotstackClient::for_test(empty, server.uri());
        let err = client
            .execute(Model::ShotstackMontage, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn server_500_is_transient() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/edit/stage/render"))
            .respond_with(ResponseTemplate::new(500).set_body_string("upstream"))
            .mount(&server)
            .await;
        let client = ShotstackClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::ShotstackMontage, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Transient(_)));
    }

    #[tokio::test]
    async fn status_401_is_auth() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/edit/stage/render"))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid key"))
            .mount(&server)
            .await;
        let client = ShotstackClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::ShotstackMontage, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn unsupported_model_is_permanent() {
        let server = MockServer::start().await;
        let client = ShotstackClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::ClaudeSonnet, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn supports_reports_only_shotstack_model() {
        let client =
            ShotstackClient::for_test(key_store_with_key(), "http://localhost".to_string());
        assert!(client.supports(Model::ShotstackMontage));
        assert!(!client.supports(Model::ClaudeSonnet));
        assert!(!client.supports(Model::IdeogramV3));
        assert!(!client.supports(Model::MeshyText3D));
        assert!(!client.supports(Model::MeshyImage3D));
    }

    // ─── assemble_timeline + poll_render ────────────────────────────────
    //
    // These cover the raw submit+poll primitives used by the
    // `shotstack_assembly` pipeline (T9). They live at the client level so
    // the pipeline tests can focus on timeline-body construction + MP4
    // cache I/O without re-asserting HTTP wire details.

    fn minimal_timeline_body() -> Value {
        json!({
            "timeline": { "tracks": [ { "clips": [] } ] },
            "output": { "format": "mp4", "resolution": "hd" }
        })
    }

    #[tokio::test]
    async fn shotstack_assembly_posts_timeline_and_returns_id() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/edit/stage/render"))
            .and(header("x-api-key", "sk-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "response": { "id": "render-xyz-789", "message": "Queued" }
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = ShotstackClient::for_test(key_store_with_key(), server.uri());
        let id = client
            .assemble_timeline(minimal_timeline_body())
            .await
            .expect("assemble_timeline succeeds");
        assert_eq!(id, "render-xyz-789");
    }

    #[tokio::test]
    async fn shotstack_assembly_polls_until_done() {
        let server = MockServer::start().await;

        // First GET reports `rendering`, then a second mount takes over with
        // `done` + URL. wiremock retires mocks whose call budget is exhausted.
        Mock::given(method("GET"))
            .and(path("/edit/stage/render/r1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "response": { "status": "rendering" }
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/edit/stage/render/r1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "response": {
                    "status": "done",
                    "url": "https://cdn.shotstack.io/out/r1.mp4"
                }
            })))
            .mount(&server)
            .await;

        let client = ShotstackClient::for_test(key_store_with_key(), server.uri());
        let body = client
            .poll_render("r1")
            .await
            .expect("poll_render succeeds");
        assert_eq!(
            body.pointer("/response/url").and_then(|v| v.as_str()),
            Some("https://cdn.shotstack.io/out/r1.mp4")
        );
        assert_eq!(
            body.pointer("/response/status").and_then(|v| v.as_str()),
            Some("done")
        );
    }

    #[tokio::test]
    async fn shotstack_assembly_propagates_failed_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/edit/stage/render/r2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "response": { "status": "failed", "error": "asset fetch failed" }
            })))
            .mount(&server)
            .await;

        let client = ShotstackClient::for_test(key_store_with_key(), server.uri());
        let err = client.poll_render("r2").await.unwrap_err();
        let s = err.to_string();
        assert!(
            matches!(err, ProviderError::Permanent(_)),
            "expected Permanent, got {err:?}"
        );
        assert!(
            s.contains("asset fetch failed"),
            "error should bubble Shotstack message, got: {s}"
        );
    }

    #[tokio::test]
    async fn shotstack_assembly_times_out_after_max_attempts() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/edit/stage/render/r3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "response": { "status": "rendering" }
            })))
            .mount(&server)
            .await;

        let client =
            ShotstackClient::for_test_with_poll_budget(key_store_with_key(), server.uri(), 2);
        let err = client.poll_render("r3").await.unwrap_err();
        // Poll exhaustion must surface as JobAlreadySubmitted (NOT Timeout)
        // so the router doesn't re-POST and create a duplicate billable job.
        match err {
            ProviderError::JobAlreadySubmitted(msg) => {
                assert!(
                    msg.contains("r3"),
                    "message should reference render id, got: {msg}"
                );
            }
            other => panic!("expected JobAlreadySubmitted, got {other:?}"),
        }
    }

    #[test]
    fn render_path_differs_per_env() {
        // Guards against a future refactor silently conflating the two.
        assert_eq!(ShotstackEnv::Stage.render_path(), "/edit/stage/render");
        assert_eq!(ShotstackEnv::Prod.render_path(), "/edit/v1/render");
        assert_ne!(
            ShotstackEnv::Stage.render_path(),
            ShotstackEnv::Prod.render_path()
        );
    }

    #[tokio::test]
    async fn prod_env_posts_to_v1_render_path() {
        let server = MockServer::start().await;
        // Only the prod path is mocked — if the client hit stage, wiremock
        // would 404 and the test would fail with ProviderError instead of Ok.
        Mock::given(method("POST"))
            .and(path("/edit/v1/render"))
            .and(header("x-api-key", "sk-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "response": { "id": "render-prod-1", "message": "Queued" }
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = ShotstackClient::for_test_with_env(
            key_store_with_key(),
            server.uri(),
            ShotstackEnv::Prod,
        );
        let id = client
            .assemble_timeline(minimal_timeline_body())
            .await
            .expect("prod assemble_timeline succeeds");
        assert_eq!(id, "render-prod-1");
    }

    #[tokio::test]
    async fn prod_env_polls_v1_render_path() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/edit/v1/render/r-prod"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "response": {
                    "status": "done",
                    "url": "https://cdn.shotstack.io/out/r-prod.mp4"
                }
            })))
            .expect(1..)
            .mount(&server)
            .await;

        let client = ShotstackClient::for_test_with_env(
            key_store_with_key(),
            server.uri(),
            ShotstackEnv::Prod,
        );
        let body = client.poll_render("r-prod").await.unwrap();
        assert_eq!(
            body.pointer("/response/url").and_then(|v| v.as_str()),
            Some("https://cdn.shotstack.io/out/r-prod.mp4")
        );
    }

    #[test]
    fn build_body_does_not_leak_user_prompt() {
        // Regression: the pre-FU #146 body stashed `_prompt` on every render
        // submission, exposing potentially sensitive user text in Shotstack
        // dashboards + logs. Assert no field references the prompt.
        let req = request("super secret user prompt");
        let body = ShotstackClient::build_body(&req);
        let serialised = body.to_string();
        assert!(
            !serialised.contains("super secret user prompt"),
            "build_body leaked user prompt into payload: {serialised}"
        );
        // Positive shape: body still has the two keys Shotstack requires.
        assert!(body.get("timeline").is_some());
        assert!(body.get("output").is_some());
    }

    #[tokio::test]
    async fn shotstack_assembly_missing_id_is_permanent() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/edit/stage/render"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "response": {}
            })))
            .mount(&server)
            .await;
        let client = ShotstackClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .assemble_timeline(minimal_timeline_body())
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }
}
