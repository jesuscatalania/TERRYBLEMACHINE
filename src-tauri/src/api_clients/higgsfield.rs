//! Higgsfield client — multi-model video generation aggregator.
//!
//! Follows the reference pattern established in [`super::claude`]:
//! 1. `new(key_store)` + `with_base_url(..)` keychain-backed constructors.
//! 2. Impl of [`AiClient`](crate::ai_router::AiClient) dispatches the single
//!    `/api/v1/generate` endpoint through `send_request`.
//! 3. Wiremock-based unit tests cover happy path + key error modes.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

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

pub struct HiggsfieldClient {
    http: Client,
    base_url: String,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
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
        }
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
        self.rate.acquire().await;
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;
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

        let url = format!("{}/api/v1/generate", self.base_url);
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

        let parsed: GenerateResponse = resp.json().await.map_err(map_reqwest_error)?;

        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({
                "job_id": parsed.id,
                "status": parsed.state,
            }),
            cost_cents: None,
            cached: false,
        })
    }
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
    async fn happy_path_returns_job_state() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/generate"))
            .and(header("x-api-key", "higgs-test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "job-abc",
                "state": "queued"
            })))
            .expect(1)
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
            "queued"
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
            .and(path("/api/v1/generate"))
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
            .and(path("/api/v1/generate"))
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
