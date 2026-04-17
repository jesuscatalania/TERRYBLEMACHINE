//! Kling AI client — text-to-video and image-to-video.
//!
//! Follows the reference pattern established in [`super::claude`]:
//! 1. `new(key_store)` + `with_base_url(..)` keychain-backed constructors.
//! 2. Impl of [`AiClient`](crate::ai_router::AiClient) dispatches the single
//!    text-to-video endpoint through `send_request`.
//! 3. Wiremock-based unit tests cover happy path + key error modes.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::common::{get_api_key, map_http_error, map_reqwest_error, RateLimiter};
use crate::ai_router::{
    AiClient, AiRequest, AiResponse, Model, Provider, ProviderError, ProviderUsage, TaskKind,
};
use crate::keychain::KeyStore;

/// Default Kling AI base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.klingai.com";
/// Keychain service id under which the Kling key lives.
pub const KEYCHAIN_SERVICE: &str = "kling";
/// Default rate: Kling caps roughly at 5 rps on the video endpoints.
const DEFAULT_RATE_PER_SEC: usize = 5;
/// Default duration (seconds) for generated videos.
const DEFAULT_DURATION_SEC: u32 = 5;

pub struct KlingClient {
    http: Client,
    base_url: String,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
}

impl KlingClient {
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
            Model::Kling20 => Some("kling-2.0"),
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

        let duration = request
            .payload
            .get("duration")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(DEFAULT_DURATION_SEC);

        let (url, body) = match request.task {
            TaskKind::TextToVideo => {
                let body = json!({
                    "model_name": slug,
                    "prompt": request.prompt,
                    "duration": duration,
                });
                let url = format!("{}/v1/videos/text2video", self.base_url);
                (url, body)
            }
            TaskKind::ImageToVideo => {
                let image_url = request
                    .payload
                    .get("image_url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ProviderError::Permanent("kling image2video: image_url required".into())
                    })?;
                let body = json!({
                    "model_name": slug,
                    "prompt": request.prompt,
                    "image_url": image_url,
                    "duration": duration,
                });
                let url = format!("{}/v1/videos/image2video", self.base_url);
                (url, body)
            }
            _ => {
                return Err(ProviderError::Permanent("kling: unsupported task".into()));
            }
        };

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

        let parsed: Text2VideoResponse = resp.json().await.map_err(map_reqwest_error)?;

        let mut output = json!({
            "job_id": parsed.task_id,
            "status": parsed.task_status,
        });
        if let Some(video_url) = parsed.video_url {
            output
                .as_object_mut()
                .expect("output is object")
                .insert("video_url".into(), json!(video_url));
        }

        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output,
            cost_cents: None,
            cached: false,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Text2VideoResponse {
    #[serde(default)]
    task_id: String,
    #[serde(default, alias = "status")]
    task_status: String,
    #[serde(default)]
    video_url: Option<String>,
}

#[async_trait]
impl AiClient for KlingClient {
    fn provider(&self) -> Provider {
        Provider::Kling
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
        // Kling health is simply "do we have a key to call with?"
        self.key_store.get(KEYCHAIN_SERVICE).is_ok()
    }

    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
        // Kling doesn't expose a stable usage endpoint — tracked via
        // per-response task metadata.
        Ok(ProviderUsage {
            notes: Some("Kling quota tracked via task polling".into()),
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
        s.store(KEYCHAIN_SERVICE, "kling-test-key").unwrap();
        Arc::new(s)
    }

    fn request(prompt: &str) -> AiRequest {
        AiRequest {
            id: "kling-r1".into(),
            task: TaskKind::TextToVideo,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: prompt.into(),
            payload: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn happy_path_returns_task_job() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/videos/text2video"))
            .and(header("Authorization", "Bearer kling-test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "task_id": "task-42",
                "task_status": "submitted"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = KlingClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(Model::Kling20, &request("a cat flying"))
            .await
            .unwrap();
        assert_eq!(resp.model, Model::Kling20);
        assert_eq!(
            resp.output.get("job_id").unwrap().as_str().unwrap(),
            "task-42"
        );
        assert_eq!(
            resp.output.get("status").unwrap().as_str().unwrap(),
            "submitted"
        );
    }

    #[tokio::test]
    async fn missing_key_yields_auth_error() {
        let server = MockServer::start().await;
        let empty = Arc::new(InMemoryStore::new()) as Arc<dyn KeyStore>;
        let client = KlingClient::for_test(empty, server.uri());
        let err = client
            .execute(Model::Kling20, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn server_500_is_transient() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/videos/text2video"))
            .respond_with(ResponseTemplate::new(500).set_body_string("upstream"))
            .mount(&server)
            .await;
        let client = KlingClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::Kling20, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Transient(_)));
    }

    #[tokio::test]
    async fn status_401_is_auth() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/videos/text2video"))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid key"))
            .mount(&server)
            .await;
        let client = KlingClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::Kling20, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn unsupported_model_is_permanent() {
        let server = MockServer::start().await;
        let client = KlingClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::ClaudeSonnet, &request("x"))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn supports_reports_only_kling_models() {
        let client = KlingClient::for_test(key_store_with_key(), "http://localhost".to_string());
        assert!(client.supports(Model::Kling20));
        assert!(!client.supports(Model::ClaudeSonnet));
        assert!(!client.supports(Model::RunwayGen3));
        assert!(!client.supports(Model::HiggsfieldMulti));
        assert!(!client.supports(Model::FalFluxPro));
    }

    #[tokio::test]
    async fn get_usage_returns_default_for_kling() {
        let client = KlingClient::for_test(key_store_with_key(), "http://localhost".to_string());
        let usage = client.get_usage().await.unwrap();
        assert!(usage.notes.is_some());
    }

    #[tokio::test]
    async fn image_to_video_hits_image2video_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/videos/image2video"))
            .and(body_partial_json(json!({
                "image_url": "https://src/a.png",
                "prompt": "cinematic"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "task_id": "t1",
                "task_status": "succeeded",
                "video_url": "https://out/v.mp4"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = KlingClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "kling-i2v-1".into(),
            task: TaskKind::ImageToVideo,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: "cinematic".into(),
            payload: json!({ "image_url": "https://src/a.png" }),
        };
        let resp = client.execute(Model::Kling20, &req).await.unwrap();
        assert_eq!(
            resp.output.get("video_url").and_then(|v| v.as_str()),
            Some("https://out/v.mp4")
        );
    }

    #[tokio::test]
    async fn image_to_video_requires_image_url() {
        let server = MockServer::start().await;
        let client = KlingClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "kling-i2v-2".into(),
            task: TaskKind::ImageToVideo,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: "x".into(),
            payload: serde_json::Value::Null,
        };
        let err = client.execute(Model::Kling20, &req).await.unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn unsupported_task_is_permanent() {
        let server = MockServer::start().await;
        let client = KlingClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "kling-bad".into(),
            task: TaskKind::ImageGeneration,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: "x".into(),
            payload: serde_json::Value::Null,
        };
        let err = client.execute(Model::Kling20, &req).await.unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }
}
