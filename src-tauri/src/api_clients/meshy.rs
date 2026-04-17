//! Meshy Pro client (Text-to-3D + Image-to-3D).
//!
//! Follows the reference pattern established by
//! [`super::claude`]: keychain-backed constructor + single `send_request`
//! pipeline + wiremock unit tests. `execute` dispatches to different
//! endpoints depending on the requested model.

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

pub struct MeshyClient {
    http: Client,
    base_url: String,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
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
        endpoint_path: &str,
        body: Value,
    ) -> Result<AiResponse, ProviderError> {
        self.rate.acquire().await;
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;

        let url = format!("{}{}", self.base_url, endpoint_path);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(key)
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

        let parsed: TaskResponse = resp.json().await.map_err(map_reqwest_error)?;

        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({
                "job_id": parsed.result,
                "status": "queued",
            }),
            cost_cents: None,
            cached: false,
        })
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
        self.send_request(model, request, TEXT_3D_PATH, body).await
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
            .unwrap_or("")
            .to_owned();
        let body = json!({
            "image_url": image_url,
        });
        self.send_request(model, request, IMAGE_3D_PATH, body).await
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
            "queued"
        );
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
