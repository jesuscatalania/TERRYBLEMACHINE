//! Shotstack JSON Timeline client (video montage, stage environment).
//!
//! Follows the reference pattern established by
//! [`super::claude`]: keychain-backed constructor + single `send_request`
//! pipeline + wiremock unit tests.

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

pub struct ShotstackClient {
    http: Client,
    base_url: String,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
}

impl ShotstackClient {
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

    /// Build the stage-render payload. Shotstack wants `{ timeline, output }`.
    /// `payload` on the request can override/extend these — for the happy-path
    /// test we fall back to a minimal sane default so the test doesn't have to
    /// stuff a whole timeline into the request body.
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

        json!({
            "timeline": timeline,
            "output": output,
            // Shotstack ignores unknown fields; we stash the prompt for
            // observability in case the timeline was derived from it.
            "callback": request.payload.get("callback").cloned(),
            "disk": "local",
            "_prompt": request.prompt,
        })
    }

    async fn send_request(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        self.rate.acquire().await;
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;

        let body = Self::build_body(request);
        let url = format!("{}/edit/stage/render", self.base_url);

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
}
