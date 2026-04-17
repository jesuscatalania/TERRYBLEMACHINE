//! Ideogram v3 client (text-in-image, logos, typography).
//!
//! Follows the reference pattern established by
//! [`super::claude`]: keychain-backed constructor + single `send_request`
//! pipeline + wiremock unit tests.

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

/// Default Ideogram base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.ideogram.ai";
/// Keychain service id under which the Ideogram key lives.
pub const KEYCHAIN_SERVICE: &str = "ideogram";
/// Default rate limit: 10 rps fits the published Ideogram quota.
const DEFAULT_RATE_PER_SEC: usize = 10;

pub struct IdeogramClient {
    http: Client,
    base_url: String,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
}

impl IdeogramClient {
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

    async fn send_request(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        self.rate.acquire().await;
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;

        let aspect_ratio = request
            .payload
            .get("aspect_ratio")
            .and_then(|v| v.as_str())
            .unwrap_or("ASPECT_1_1")
            .to_owned();

        let body = json!({
            "image_request": {
                "prompt": request.prompt,
                "aspect_ratio": aspect_ratio,
            }
        });

        let url = format!("{}/generate", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header("Api-Key", key)
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
        let first = parsed.data.into_iter().next().unwrap_or_default();

        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({
                "job_id": first.url.clone(),
                "status": "completed",
                "url": first.url,
                "prompt": first.prompt,
                "resolution": first.resolution,
            }),
            cost_cents: None,
            cached: false,
        })
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct GenerateResponse {
    #[serde(default)]
    data: Vec<GeneratedImage>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct GeneratedImage {
    #[serde(default)]
    url: String,
    #[serde(default)]
    prompt: String,
    #[serde(default)]
    resolution: String,
}

#[async_trait]
impl AiClient for IdeogramClient {
    fn provider(&self) -> Provider {
        Provider::Ideogram
    }

    fn supports(&self, model: Model) -> bool {
        matches!(model, Model::IdeogramV3)
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
            notes: Some("Ideogram usage tracked via dashboard".into()),
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
            task: TaskKind::Logo,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: prompt.into(),
            payload: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn happy_path_returns_url() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/generate"))
            .and(header("Api-Key", "sk-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [{
                    "url": "https://ideogram.ai/img/abc.png",
                    "prompt": "logo for Acme",
                    "resolution": "1024x1024"
                }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = IdeogramClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(Model::IdeogramV3, &request("logo for Acme"))
            .await
            .unwrap();
        assert_eq!(resp.model, Model::IdeogramV3);
        assert_eq!(
            resp.output.get("url").unwrap().as_str().unwrap(),
            "https://ideogram.ai/img/abc.png"
        );
        assert_eq!(
            resp.output.get("resolution").unwrap().as_str().unwrap(),
            "1024x1024"
        );
    }

    #[tokio::test]
    async fn missing_key_yields_auth_error() {
        let server = MockServer::start().await;
        let empty = Arc::new(InMemoryStore::new()) as Arc<dyn KeyStore>;
        let client = IdeogramClient::for_test(empty, server.uri());
        let err = client
            .execute(Model::IdeogramV3, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn server_500_is_transient() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/generate"))
            .respond_with(ResponseTemplate::new(500).set_body_string("upstream"))
            .mount(&server)
            .await;
        let client = IdeogramClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::IdeogramV3, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Transient(_)));
    }

    #[tokio::test]
    async fn status_401_is_auth() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/generate"))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid key"))
            .mount(&server)
            .await;
        let client = IdeogramClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::IdeogramV3, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn unsupported_model_is_permanent() {
        let server = MockServer::start().await;
        let client = IdeogramClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::ClaudeSonnet, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn supports_reports_only_ideogram_model() {
        let client = IdeogramClient::for_test(key_store_with_key(), "http://localhost".to_string());
        assert!(client.supports(Model::IdeogramV3));
        assert!(!client.supports(Model::ClaudeSonnet));
        assert!(!client.supports(Model::ShotstackMontage));
        assert!(!client.supports(Model::MeshyText3D));
        assert!(!client.supports(Model::MeshyImage3D));
    }
}
