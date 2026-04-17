//! Runway Gen-3 client — image-to-video generation.
//!
//! Follows the reference pattern established in [`super::claude`]:
//! 1. `new(key_store)` + `with_base_url(..)` keychain-backed constructors.
//! 2. Impl of [`AiClient`](crate::ai_router::AiClient) dispatches the single
//!    image-to-video endpoint through `send_request`.
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

/// Default Runway API base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.dev.runwayml.com";
/// Keychain service id under which the Runway key lives.
pub const KEYCHAIN_SERVICE: &str = "runway";
/// Default rate limit — Runway Pro allows ~5 rps on the generation endpoints.
const DEFAULT_RATE_PER_SEC: usize = 5;

pub struct RunwayClient {
    http: Client,
    base_url: String,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
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
            Model::RunwayGen3 => Some("gen3"),
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
        // (e.g. `{ "strokes": [...] }`).
        if let Some(motion_brush) = request.payload.get("motion_brush") {
            body.as_object_mut()
                .expect("body is a JSON object")
                .insert("motion_brush".into(), motion_brush.clone());
        }

        let url = format!("{}/v1/image_to_video", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {key}"))
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

        let parsed: ImageToVideoResponse = resp.json().await.map_err(map_reqwest_error)?;

        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({
                "job_id": parsed.id,
                "status": parsed.status,
            }),
            cost_cents: None,
            cached: false,
        })
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
        }
    }

    #[tokio::test]
    async fn happy_path_returns_job_id() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/image_to_video"))
            .and(header("Authorization", "Bearer runway-test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "gen-77",
                "status": "PENDING"
            })))
            .expect(1)
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
            "PENDING"
        );
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
            .and(path("/v1/image_to_video"))
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
            .and(path("/v1/image_to_video"))
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
    async fn motion_brush_strokes_forwarded() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/image_to_video"))
            .and(body_partial_json(json!({
                "motion_brush": { "strokes": [{"x": 10, "y": 20, "dx": 5, "dy": 0}] }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "r1",
                "status": "succeeded",
                "output": ["https://out/v.mp4"]
            })))
            .expect(1)
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
        };
        let resp = client.execute(Model::RunwayGen3, &req).await.unwrap();
        assert_eq!(
            resp.output.get("job_id").and_then(|v| v.as_str()),
            Some("r1")
        );
    }
}
