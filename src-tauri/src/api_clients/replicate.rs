//! Replicate API client — a single `/v1/predictions` endpoint that creates a
//! prediction for a versioned model (e.g. Flux-Dev).
//!
//! Follows the reference pattern in [`super::claude`]:
//! 1. `new` / `with_base_url` / `for_test` constructors.
//! 2. A private `send_request` that acquires a rate-limit permit, loads the
//!    API key, POSTs `{ version, input }`, and maps HTTP / reqwest errors via
//!    `common`.
//! 3. `impl AiClient` supports only [`Model::ReplicateFluxDev`].

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

pub struct ReplicateClient {
    http: Client,
    base_url: String,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
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
        }
    }

    /// Test-only constructor that skips the refill task.
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
        }
    }

    fn version_for(model: Model) -> Option<&'static str> {
        match model {
            Model::ReplicateFluxDev => Some(FLUX_DEV_VERSION),
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
        let version = Self::version_for(model)
            .ok_or_else(|| ProviderError::Permanent(format!("unsupported model {model:?}")))?;

        let body = json!({
            "version": version,
            "input": { "prompt": request.prompt },
        });

        let url = format!("{}/v1/predictions", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header("authorization", format!("Token {key}"))
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

        let parsed: PredictionResponse = resp.json().await.map_err(map_reqwest_error)?;
        let output = json!({
            "id": parsed.id,
            "status": parsed.status,
            "urls": parsed.urls,
            "output": parsed.output,
        });

        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output,
            cost_cents: None,
            cached: false,
        })
    }
}

// ─── Response types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PredictionUrls {
    #[serde(default)]
    get: Option<String>,
    #[serde(default)]
    cancel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PredictionResponse {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    urls: Option<PredictionUrls>,
    #[serde(default)]
    output: Option<Value>,
}

#[async_trait]
impl AiClient for ReplicateClient {
    fn provider(&self) -> Provider {
        Provider::Replicate
    }

    fn supports(&self, model: Model) -> bool {
        Self::version_for(model).is_some()
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
                "status": "starting",
                "urls": {
                    "get": "https://api.replicate.com/v1/predictions/pred-123",
                    "cancel": "https://api.replicate.com/v1/predictions/pred-123/cancel",
                },
                "output": null,
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
            "starting"
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
        assert!(!client.supports(Model::FalFluxPro));
        assert!(!client.supports(Model::ClaudeSonnet));
        assert!(!client.supports(Model::Kling20));
    }
}
