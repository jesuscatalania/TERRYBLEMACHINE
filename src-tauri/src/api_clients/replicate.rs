//! Replicate API client — creates predictions and polls until completion.
//!
//! Supports two dispatch shapes:
//! 1. **Version-pinned**: `POST /v1/predictions` with `{ version, input }`.
//!    Used for [`Model::ReplicateFluxDev`] where we want reproducibility
//!    across a specific Flux-Dev weight release.
//! 2. **Slug-based**: `POST /v1/models/<owner>/<name>/predictions` with
//!    `{ input }` — always routes to the model's default version and avoids
//!    hardcoded version-hash maintenance. Used for
//!    [`Model::ReplicateDepthAnythingV2`] and [`Model::ReplicateTripoSR`].
//!
//! Replicate responses come in three phases:
//! 1. 201 Created: `{ id, status: "starting", urls: { get }, ... }`.
//! 2. GET poll: `{ status: "processing" }` — still working.
//! 3. GET poll: `{ status: "succeeded", output: ... }` — done.
//!
//! With `Prefer: wait` the initial POST blocks up to 60s and often inlines
//! `succeeded`, covering the fast path. Longer jobs (TripoSR in particular)
//! need a follow-up polling loop — mirrors Meshy's T8 pattern.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

use super::common::{get_api_key, map_http_error, map_reqwest_error, RateLimiter};
use crate::ai_router::{
    AiClient, AiRequest, AiResponse, Model, Provider, ProviderError, ProviderUsage,
};
use crate::keychain::KeyStore;

/// Default base URL for the Replicate REST API.
pub const DEFAULT_BASE_URL: &str = "https://api.replicate.com";
/// Keychain service id under which the Replicate key lives.
pub const KEYCHAIN_SERVICE: &str = "replicate";
/// Default rate: Replicate accepts ~10 rps on the predictions endpoint.
const DEFAULT_RATE_PER_SEC: usize = 10;

/// Pinned version hash for Black Forest Labs' Flux-Dev on Replicate. In
/// production this should be sourced from config or a version-lookup call;
/// hard-coding it here keeps the client self-contained for Schritt 2.2.
const FLUX_DEV_VERSION: &str = "f2ab8a5569070ef6f6b2f0ede5a3f1a7fbfe0a5e1f6fb1bdf7d55c1e0e1b1b0b";

/// Replicate model slugs. Slug-based endpoints avoid version-hash maintenance
/// but require verifying the slug exists at https://replicate.com/<slug>.
const DEPTH_ANYTHING_V2_SLUG: &str = "depth-anything/depth-anything-v2-large";
/// TripoSR slug. TODO(phase-5): verify against
/// <https://replicate.com/camenduru/tripo-sr>; the owner may vary across
/// community mirrors (e.g. `tripo3d/tripo`).
const TRIPO_SR_SLUG: &str = "camenduru/tripo-sr";

/// Upper bound for prediction-status poll iterations. With exponential
/// back-off from 2s to 15s this sums to roughly 6 minutes of waiting before
/// we surrender and bubble a `Timeout` to the router. Matches Meshy's
/// `poll_task` shape (T8) — TripoSR jobs can take longer than Flux/Depth so
/// the ceiling is a bit higher than Meshy's 20-attempt default.
const DEFAULT_REPLICATE_POLL_MAX_ATTEMPTS: u32 = 30;
const DEFAULT_REPLICATE_POLL_INITIAL_DELAY: Duration = Duration::from_secs(2);
const DEFAULT_REPLICATE_POLL_MAX_DELAY: Duration = Duration::from_secs(15);

pub struct ReplicateClient {
    http: Client,
    base_url: String,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
    poll_max_attempts: u32,
    poll_initial_delay: Duration,
    poll_max_delay: Duration,
}

/// Dispatch shape for a given [`Model`]: either a pinned version hash or a
/// stable `owner/name` slug.
enum Endpoint {
    Version(&'static str),
    Slug(&'static str),
}

impl ReplicateClient {
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
            poll_max_attempts: DEFAULT_REPLICATE_POLL_MAX_ATTEMPTS,
            poll_initial_delay: DEFAULT_REPLICATE_POLL_INITIAL_DELAY,
            poll_max_delay: DEFAULT_REPLICATE_POLL_MAX_DELAY,
        }
    }

    /// Test-only constructor that skips the refill task and collapses poll
    /// delays so tests don't sit spinning for minutes.
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
            poll_max_attempts: DEFAULT_REPLICATE_POLL_MAX_ATTEMPTS,
            poll_initial_delay: Duration::from_millis(10),
            poll_max_delay: Duration::from_millis(100),
        }
    }

    /// Like [`Self::for_test`], with a custom cap on poll attempts. Used by
    /// the timeout-specific test so we fail fast when a prediction never
    /// reaches a terminal state.
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

    fn endpoint_for(model: Model) -> Option<Endpoint> {
        match model {
            Model::ReplicateFluxDev => Some(Endpoint::Version(FLUX_DEV_VERSION)),
            Model::ReplicateDepthAnythingV2 => Some(Endpoint::Slug(DEPTH_ANYTHING_V2_SLUG)),
            Model::ReplicateTripoSR => Some(Endpoint::Slug(TRIPO_SR_SLUG)),
            _ => None,
        }
    }

    /// Shape the `input` object for a given model. Replicate's predictions
    /// endpoint takes `input` whose schema is model-specific.
    fn input_for(model: Model, request: &AiRequest) -> Result<Value, ProviderError> {
        match model {
            Model::ReplicateFluxDev => Ok(json!({ "prompt": request.prompt })),
            Model::ReplicateDepthAnythingV2 => {
                let image_url = request
                    .payload
                    .get("image_url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ProviderError::Permanent(
                            "depth-anything: image_url required in payload".into(),
                        )
                    })?;
                Ok(json!({ "image": image_url }))
            }
            Model::ReplicateTripoSR => {
                let image_url = request
                    .payload
                    .get("image_url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ProviderError::Permanent("triposr: image_url required in payload".into())
                    })?;
                // TripoSR on Replicate accepts an `image` input (path or URL).
                Ok(json!({ "image": image_url }))
            }
            _ => Err(ProviderError::Permanent(format!(
                "unsupported model {model:?}"
            ))),
        }
    }

    async fn send_request(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        self.rate.acquire().await;
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;
        let endpoint = Self::endpoint_for(model)
            .ok_or_else(|| ProviderError::Permanent(format!("unsupported model {model:?}")))?;
        let input = Self::input_for(model, request)?;

        let (url, body) = match endpoint {
            Endpoint::Version(version) => (
                format!("{}/v1/predictions", self.base_url),
                json!({ "version": version, "input": input }),
            ),
            Endpoint::Slug(slug) => (
                format!("{}/v1/models/{slug}/predictions", self.base_url),
                json!({ "input": input }),
            ),
        };

        // `Prefer: wait` asks Replicate to block up to 60s for the prediction
        // to finish and inline the result in the response. Fast models return
        // directly; longer ones (TripoSR) still come back with a "starting"
        // or "processing" status and need the polling fallback below.
        let resp = self
            .http
            .post(&url)
            .header("authorization", format!("Token {key}"))
            .header("content-type", "application/json")
            .header("Prefer", "wait")
            .json(&body)
            .send()
            .await
            .map_err(map_reqwest_error)?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(map_http_error(status, &text));
        }

        let initial: Value = resp.json().await.map_err(map_reqwest_error)?;

        // If Replicate returns a terminal state inline, keep it. Otherwise
        // poll `urls.get` until we reach one.
        let final_body = match initial.get("status").and_then(|v| v.as_str()) {
            Some("succeeded") | Some("failed") | Some("canceled") => initial,
            _ => {
                let get_url = initial
                    .get("urls")
                    .and_then(|u| u.get("get"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ProviderError::Permanent(
                            "replicate: response missing urls.get for polling".into(),
                        )
                    })?
                    .to_string();
                self.poll_prediction(&get_url).await?
            }
        };

        // Guard the terminal-status check in case `poll_prediction` returns a
        // non-succeeded body (it shouldn't — but a belt-and-braces check keeps
        // the error surface honest for callers).
        match final_body.get("status").and_then(|v| v.as_str()) {
            Some("succeeded") => {}
            Some("failed") | Some("canceled") => {
                let msg = final_body
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("replicate prediction failed");
                return Err(ProviderError::Permanent(msg.to_string()));
            }
            other => {
                return Err(ProviderError::Permanent(format!(
                    "replicate: unexpected terminal status: {other:?}"
                )));
            }
        }

        let output_value = final_body.get("output").cloned().unwrap_or(Value::Null);
        let id = final_body
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        let urls = final_body.get("urls").cloned();

        let mut output = json!({
            "id": id,
            "status": "succeeded",
            "urls": urls,
            "output": output_value,
        });

        // TripoSR returns a single GLB URL under `output`. The mesh pipeline
        // extracts meshes via `output.glb_url`, so surface the URL under
        // that key too for a frictionless hand-off.
        if matches!(model, Model::ReplicateTripoSR) {
            if let Some(url) = final_body
                .get("output")
                .and_then(|v| v.as_str())
                .map(str::to_owned)
            {
                if let Some(obj) = output.as_object_mut() {
                    obj.insert("glb_url".into(), Value::String(url));
                }
            }
        }

        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output,
            cost_cents: None,
            cached: false,
        })
    }

    /// Poll `GET <get_url>` (the `urls.get` field from the initial POST) until
    /// Replicate reports a terminal status. Returns the final JSON body on
    /// `succeeded`; maps `failed` / `canceled` to [`ProviderError::Permanent`]
    /// (no retry — the content was rejected) and exhausted attempts to
    /// [`ProviderError::Timeout`] (transient — router may fall back).
    async fn poll_prediction(&self, get_url: &str) -> Result<Value, ProviderError> {
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;
        let mut delay = self.poll_initial_delay;

        for _ in 0..self.poll_max_attempts {
            // Rate-limit every poll so concurrent predictions can't burst GETs
            // past Replicate's per-second cap. `RateLimiter::unlimited()` (used
            // by `for_test*`) is a zero-cost no-op in tests.
            self.rate.acquire().await;
            let resp = self
                .http
                .get(get_url)
                .header("authorization", format!("Token {}", &key))
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
                Some("succeeded") => return Ok(body),
                Some("failed") | Some("canceled") => {
                    let msg = body
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("replicate prediction failed");
                    return Err(ProviderError::Permanent(msg.to_string()));
                }
                // starting / processing / unknown — keep polling.
                _ => {
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(self.poll_max_delay);
                }
            }
        }

        Err(ProviderError::Timeout)
    }
}

#[async_trait]
impl AiClient for ReplicateClient {
    fn provider(&self) -> Provider {
        Provider::Replicate
    }

    fn supports(&self, model: Model) -> bool {
        Self::endpoint_for(model).is_some()
    }

    async fn execute(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        self.send_request(model, request).await
    }

    async fn health_check(&self) -> bool {
        self.key_store.get(KEYCHAIN_SERVICE).is_ok()
    }

    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
        // Replicate's account endpoint exposes credit balance; we don't call
        // it on every get_usage to avoid side-effects in Schritt 2.2.
        Ok(ProviderUsage {
            notes: Some("replicate usage tracked via /v1/account".into()),
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
        s.store(KEYCHAIN_SERVICE, "r8-test").unwrap();
        Arc::new(s)
    }

    fn request(prompt: &str) -> AiRequest {
        AiRequest {
            id: "r1".into(),
            task: TaskKind::ImageGeneration,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: prompt.into(),
            payload: Value::Null,
        }
    }

    #[tokio::test]
    async fn happy_path_returns_prediction() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/predictions"))
            .and(header("authorization", "Token r8-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "pred-123",
                "status": "succeeded",
                "urls": {
                    "get": "https://api.replicate.com/v1/predictions/pred-123",
                    "cancel": "https://api.replicate.com/v1/predictions/pred-123/cancel",
                },
                "output": "https://fake.replicate/out.png",
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = ReplicateClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(Model::ReplicateFluxDev, &request("a robot"))
            .await
            .unwrap();
        assert_eq!(resp.model, Model::ReplicateFluxDev);
        assert_eq!(resp.output.get("id").unwrap().as_str().unwrap(), "pred-123");
        assert_eq!(
            resp.output.get("status").unwrap().as_str().unwrap(),
            "succeeded"
        );
        assert_eq!(
            resp.output
                .get("urls")
                .and_then(|u| u.get("get"))
                .unwrap()
                .as_str()
                .unwrap(),
            "https://api.replicate.com/v1/predictions/pred-123"
        );
    }

    #[tokio::test]
    async fn missing_key_yields_auth_error() {
        let server = MockServer::start().await;
        let empty = Arc::new(InMemoryStore::new()) as Arc<dyn KeyStore>;
        let client = ReplicateClient::for_test(empty, server.uri());
        let err = client
            .execute(Model::ReplicateFluxDev, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn server_500_is_transient() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/predictions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("upstream"))
            .mount(&server)
            .await;
        let client = ReplicateClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::ReplicateFluxDev, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Transient(_)));
    }

    #[tokio::test]
    async fn status_401_is_auth() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/predictions"))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid token"))
            .mount(&server)
            .await;
        let client = ReplicateClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::ReplicateFluxDev, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn unsupported_model_is_permanent() {
        let server = MockServer::start().await;
        let client = ReplicateClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::ClaudeSonnet, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn supports_reports_only_replicate_models() {
        let client =
            ReplicateClient::for_test(key_store_with_key(), "http://localhost".to_string());
        assert!(client.supports(Model::ReplicateFluxDev));
        assert!(client.supports(Model::ReplicateDepthAnythingV2));
        assert!(!client.supports(Model::FalFluxPro));
        assert!(!client.supports(Model::ClaudeSonnet));
        assert!(!client.supports(Model::Kling20));
    }

    #[tokio::test]
    async fn depth_anything_v2_posts_to_slug_endpoint() {
        use wiremock::matchers::body_partial_json;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/v1/models/depth-anything/depth-anything-v2-large/predictions",
            ))
            .and(header("authorization", "Token r8-test"))
            .and(header("Prefer", "wait"))
            .and(body_partial_json(json!({
                "input": { "image": "https://src/a.png" },
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "pred-depth-1",
                "status": "succeeded",
                "urls": {
                    "get": "https://api.replicate.com/v1/predictions/pred-depth-1",
                    "cancel": "https://api.replicate.com/v1/predictions/pred-depth-1/cancel",
                },
                "output": "https://fake.replicate/depth.png",
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = ReplicateClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "depth-r1".into(),
            task: TaskKind::DepthMap,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: String::new(),
            payload: json!({ "image_url": "https://src/a.png" }),
        };
        let resp = client
            .execute(Model::ReplicateDepthAnythingV2, &req)
            .await
            .expect("depth prediction succeeds");

        assert_eq!(resp.model, Model::ReplicateDepthAnythingV2);
        assert_eq!(
            resp.output
                .get("output")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            "https://fake.replicate/depth.png"
        );
    }

    #[tokio::test]
    async fn triposr_posts_to_slug_endpoint() {
        use wiremock::matchers::body_partial_json;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/models/camenduru/tripo-sr/predictions"))
            .and(header("authorization", "Token r8-test"))
            .and(header("Prefer", "wait"))
            .and(body_partial_json(json!({
                "input": { "image": "https://src/a.png" },
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "pred-triposr-1",
                "status": "succeeded",
                "urls": {
                    "get": "https://api.replicate.com/v1/predictions/pred-triposr-1",
                    "cancel": "https://api.replicate.com/v1/predictions/pred-triposr-1/cancel",
                },
                "output": "https://fake.replicate/model.glb",
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = ReplicateClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "triposr-r1".into(),
            task: TaskKind::Image3D,
            priority: Priority::Normal,
            complexity: Complexity::Simple,
            prompt: String::new(),
            payload: json!({ "image_url": "https://src/a.png" }),
        };
        let resp = client
            .execute(Model::ReplicateTripoSR, &req)
            .await
            .expect("triposr prediction succeeds");

        assert_eq!(resp.model, Model::ReplicateTripoSR);
        // The mesh pipeline reads `output.glb_url` — make sure we surface it.
        assert_eq!(
            resp.output
                .get("glb_url")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            "https://fake.replicate/model.glb"
        );
        // And the raw `output` passthrough is still there for parity.
        assert_eq!(
            resp.output
                .get("output")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            "https://fake.replicate/model.glb"
        );
    }

    #[tokio::test]
    async fn triposr_requires_image_url() {
        let server = MockServer::start().await;
        let client = ReplicateClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "triposr-bad".into(),
            task: TaskKind::Image3D,
            priority: Priority::Normal,
            complexity: Complexity::Simple,
            prompt: String::new(),
            payload: json!({}),
        };
        let err = client
            .execute(Model::ReplicateTripoSR, &req)
            .await
            .expect_err("missing image_url must be a Permanent error");
        assert!(matches!(err, ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn supports_includes_triposr() {
        let client =
            ReplicateClient::for_test(key_store_with_key(), "http://localhost".to_string());
        assert!(client.supports(Model::ReplicateTripoSR));
    }

    #[tokio::test]
    async fn depth_anything_v2_requires_image_url() {
        let server = MockServer::start().await;
        let client = ReplicateClient::for_test(key_store_with_key(), server.uri());
        // No image_url in payload → client must fail fast, not hit the server.
        let req = AiRequest {
            id: "depth-bad".into(),
            task: TaskKind::DepthMap,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: String::new(),
            payload: json!({}),
        };
        let err = client
            .execute(Model::ReplicateDepthAnythingV2, &req)
            .await
            .expect_err("missing image_url must be a Permanent error");
        assert!(matches!(err, ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn replicate_polls_starting_prediction_until_succeeded() {
        use wiremock::matchers::body_partial_json;

        let server = MockServer::start().await;

        // Initial POST returns status=starting with urls.get pointing at our
        // mock server. No output yet — this is the bug FU #129 reported:
        // without polling the client would treat this as NoOutput.
        let poll_url = format!("{}/v1/predictions/pred-polling-1", server.uri());
        Mock::given(method("POST"))
            .and(path("/v1/models/camenduru/tripo-sr/predictions"))
            .and(header("Prefer", "wait"))
            .and(body_partial_json(json!({
                "input": { "image": "https://src/a.png" },
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "id": "pred-polling-1",
                "status": "starting",
                "urls": { "get": poll_url.clone() },
                "output": null,
            })))
            .expect(1)
            .mount(&server)
            .await;

        // First GET: still processing.
        Mock::given(method("GET"))
            .and(path("/v1/predictions/pred-polling-1"))
            .and(header("authorization", "Token r8-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "pred-polling-1",
                "status": "processing",
                "output": null,
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Subsequent GETs: succeeded with the GLB URL.
        Mock::given(method("GET"))
            .and(path("/v1/predictions/pred-polling-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "pred-polling-1",
                "status": "succeeded",
                "urls": { "get": poll_url.clone() },
                "output": "https://fake.replicate/polled.glb",
            })))
            .mount(&server)
            .await;

        let client = ReplicateClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "triposr-poll".into(),
            task: TaskKind::Image3D,
            priority: Priority::Normal,
            complexity: Complexity::Simple,
            prompt: String::new(),
            payload: json!({ "image_url": "https://src/a.png" }),
        };
        let resp = client
            .execute(Model::ReplicateTripoSR, &req)
            .await
            .expect("polled prediction succeeds");

        assert_eq!(resp.model, Model::ReplicateTripoSR);
        assert_eq!(
            resp.output
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            "succeeded"
        );
        assert_eq!(
            resp.output
                .get("output")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            "https://fake.replicate/polled.glb"
        );
        assert_eq!(
            resp.output
                .get("glb_url")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            "https://fake.replicate/polled.glb"
        );
    }
}
