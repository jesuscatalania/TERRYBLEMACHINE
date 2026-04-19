//! fal.ai API client — image generation, upscale, and inpaint via the
//! `queue.fal.run` queue endpoints.
//!
//! Follows the reference pattern in [`super::claude`]:
//! 1. `new` / `with_base_url` / `for_test` constructors.
//! 2. A private `send_request` that acquires a rate-limit permit, loads the
//!    API key, POSTs to the model-specific endpoint, and maps HTTP / reqwest
//!    errors via `common`.
//! 3. `impl AiClient` dispatches on [`Model`] to the correct endpoint +
//!    request payload, and unpacks the model-specific response JSON.

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

/// Default base URL for the fal.ai queue endpoints.
pub const DEFAULT_BASE_URL: &str = "https://queue.fal.run";
/// Keychain service id under which the fal.ai key lives.
pub const KEYCHAIN_SERVICE: &str = "fal";
/// Default rate: fal.ai free tier is lenient; 10 rps is a safe ceiling.
const DEFAULT_RATE_PER_SEC: usize = 10;

pub struct FalClient {
    http: Client,
    base_url: String,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
}

impl FalClient {
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

    /// The endpoint path for a given supported model.
    //
    // Naming note: the `Model::FalFluxPro` variant maps to the fal.ai
    // endpoint for Flux Pro (marketed as "Flux 1.1 Pro"). The plan doc
    // uses "Flux 2 Pro" as an aspirational name; when fal.ai ships a v2
    // endpoint, add the new `Model` variant and a new arm here.
    fn endpoint_for(model: Model) -> Option<&'static str> {
        match model {
            Model::FalFluxPro => Some("/fal-ai/flux-pro"),
            Model::FalSdxl => Some("/fal-ai/fast-sdxl"),
            Model::FalRealEsrgan => Some("/fal-ai/real-esrgan"),
            Model::FalFluxFill => Some("/fal-ai/flux-fill"),
            _ => None,
        }
    }

    /// Build the request body for a given model and [`AiRequest`].
    ///
    /// The exact shape is endpoint-specific; we pull image URLs / scale /
    /// mask URL out of `request.payload` where relevant.
    fn body_for(model: Model, request: &AiRequest) -> Result<Value, ProviderError> {
        match model {
            Model::FalFluxPro => {
                let image_size = request
                    .payload
                    .get("image_size")
                    .cloned()
                    .unwrap_or_else(|| json!("landscape_4_3"));
                Ok(json!({
                    "prompt": request.prompt,
                    "image_size": image_size,
                }))
            }
            Model::FalSdxl => Ok(json!({ "prompt": request.prompt })),
            Model::FalRealEsrgan => {
                let image_url = request
                    .payload
                    .get("image_url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ProviderError::Permanent(
                            "fal real-esrgan: payload.image_url is required".into(),
                        )
                    })?;
                let scale = request
                    .payload
                    .get("scale")
                    .cloned()
                    .unwrap_or_else(|| json!(2));
                Ok(json!({
                    "image_url": image_url,
                    "scale": scale,
                }))
            }
            Model::FalFluxFill => {
                let image_url = request
                    .payload
                    .get("image_url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ProviderError::Permanent(
                            "fal flux-fill: payload.image_url is required".into(),
                        )
                    })?;
                let mask_url = request
                    .payload
                    .get("mask_url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ProviderError::Permanent(
                            "fal flux-fill: payload.mask_url is required".into(),
                        )
                    })?;
                Ok(json!({
                    "image_url": image_url,
                    "mask_url": mask_url,
                    "prompt": request.prompt,
                }))
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
        let body = Self::body_for(model, request)?;

        let url = format!("{}{}", self.base_url, endpoint);
        let resp = self
            .http
            .post(&url)
            .header("authorization", format!("Key {key}"))
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

        let output = match model {
            Model::FalFluxPro => {
                let parsed: FluxProResponse = resp.json().await.map_err(map_reqwest_error)?;
                let first = parsed.images.first();
                json!({
                    "images": parsed.images,
                    "seed": parsed.seed,
                    "url": first.map(|i| i.url.clone()),
                })
            }
            Model::FalSdxl => {
                let parsed: SdxlResponse = resp.json().await.map_err(map_reqwest_error)?;
                let first = parsed.images.first();
                json!({
                    "images": parsed.images,
                    "seed": parsed.seed,
                    "url": first.map(|i| i.url.clone()),
                })
            }
            Model::FalRealEsrgan => {
                let parsed: RealEsrganResponse = resp.json().await.map_err(map_reqwest_error)?;
                json!({
                    "image": parsed.image,
                    "url": parsed.image.url.clone(),
                })
            }
            Model::FalFluxFill => {
                let parsed: FluxFillResponse = resp.json().await.map_err(map_reqwest_error)?;
                let first = parsed.images.first();
                json!({
                    "images": parsed.images,
                    "url": first.map(|i| i.url.clone()),
                })
            }
            _ => unreachable!("endpoint_for guards the model set"),
        };

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
struct ImageDescriptor {
    url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FluxProResponse {
    #[serde(default)]
    images: Vec<ImageDescriptor>,
    #[serde(default)]
    seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SdxlResponse {
    #[serde(default)]
    images: Vec<ImageDescriptor>,
    #[serde(default)]
    seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RealEsrganResponse {
    image: ImageDescriptor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FluxFillResponse {
    #[serde(default)]
    images: Vec<ImageDescriptor>,
}

#[async_trait]
impl AiClient for FalClient {
    fn provider(&self) -> Provider {
        Provider::Fal
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
        // fal.ai does not expose a usage endpoint; callers track via
        // dashboard / webhook-reported credits.
        Ok(ProviderUsage {
            notes: Some("fal.ai usage tracked via dashboard".into()),
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
        s.store(KEYCHAIN_SERVICE, "fal-test").unwrap();
        Arc::new(s)
    }

    fn request(prompt: &str, payload: Value) -> AiRequest {
        AiRequest {
            id: "r1".into(),
            task: TaskKind::ImageGeneration,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: prompt.into(),
            payload,
        }
    }

    // ─── Happy paths (one per model) ──────────────────────────────────

    #[tokio::test]
    async fn flux_pro_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/fal-ai/flux-pro"))
            .and(header("authorization", "Key fal-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "images": [{ "url": "https://cdn/img.png", "width": 1024, "height": 768 }],
                "seed": 42,
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = FalClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(Model::FalFluxPro, &request("a cat", Value::Null))
            .await
            .unwrap();
        assert_eq!(resp.model, Model::FalFluxPro);
        assert_eq!(
            resp.output.get("url").unwrap().as_str().unwrap(),
            "https://cdn/img.png"
        );
        assert_eq!(resp.output.get("seed").unwrap().as_u64().unwrap(), 42);
    }

    #[tokio::test]
    async fn sdxl_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/fal-ai/fast-sdxl"))
            .and(header("authorization", "Key fal-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "images": [{ "url": "https://cdn/sdxl.png" }],
                "seed": 7,
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = FalClient::for_test(key_store_with_key(), server.uri());
        let resp = client
            .execute(Model::FalSdxl, &request("a sunset", Value::Null))
            .await
            .unwrap();
        assert_eq!(resp.model, Model::FalSdxl);
        assert_eq!(
            resp.output.get("url").unwrap().as_str().unwrap(),
            "https://cdn/sdxl.png"
        );
        assert_eq!(resp.output.get("seed").unwrap().as_u64().unwrap(), 7);
    }

    #[tokio::test]
    async fn real_esrgan_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/fal-ai/real-esrgan"))
            .and(header("authorization", "Key fal-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "image": { "url": "https://cdn/up.png", "width": 2048, "height": 1536 }
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = FalClient::for_test(key_store_with_key(), server.uri());
        let payload = json!({ "image_url": "https://src/in.png", "scale": 4 });
        let resp = client
            .execute(Model::FalRealEsrgan, &request("", payload))
            .await
            .unwrap();
        assert_eq!(resp.model, Model::FalRealEsrgan);
        assert_eq!(
            resp.output.get("url").unwrap().as_str().unwrap(),
            "https://cdn/up.png"
        );
    }

    #[tokio::test]
    async fn flux_fill_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/fal-ai/flux-fill"))
            .and(header("authorization", "Key fal-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "images": [{ "url": "https://cdn/fill.png" }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = FalClient::for_test(key_store_with_key(), server.uri());
        let payload = json!({
            "image_url": "https://src/in.png",
            "mask_url": "https://src/mask.png"
        });
        let resp = client
            .execute(Model::FalFluxFill, &request("add a hat", payload))
            .await
            .unwrap();
        assert_eq!(resp.model, Model::FalFluxFill);
        assert_eq!(
            resp.output.get("url").unwrap().as_str().unwrap(),
            "https://cdn/fill.png"
        );
    }

    // ─── Error / support tests ────────────────────────────────────────

    #[tokio::test]
    async fn missing_key_yields_auth_error() {
        let server = MockServer::start().await;
        let empty = Arc::new(InMemoryStore::new()) as Arc<dyn KeyStore>;
        let client = FalClient::for_test(empty, server.uri());
        let err = client
            .execute(Model::FalSdxl, &request("x", Value::Null))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn server_500_is_transient() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/fal-ai/fast-sdxl"))
            .respond_with(ResponseTemplate::new(500).set_body_string("oops"))
            .mount(&server)
            .await;
        let client = FalClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::FalSdxl, &request("x", Value::Null))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Transient(_)));
    }

    #[tokio::test]
    async fn status_401_is_auth() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/fal-ai/fast-sdxl"))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid key"))
            .mount(&server)
            .await;
        let client = FalClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::FalSdxl, &request("x", Value::Null))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn unsupported_model_is_permanent() {
        let server = MockServer::start().await;
        let client = FalClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::ClaudeSonnet, &request("x", Value::Null))
            .await
            .unwrap_err();
        assert!(matches!(err, ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn supports_reports_only_fal_models() {
        let client = FalClient::for_test(key_store_with_key(), "http://localhost".to_string());
        assert!(client.supports(Model::FalFluxPro));
        assert!(client.supports(Model::FalSdxl));
        assert!(client.supports(Model::FalRealEsrgan));
        assert!(client.supports(Model::FalFluxFill));
        assert!(!client.supports(Model::ClaudeSonnet));
        assert!(!client.supports(Model::ReplicateFluxDev));
        assert!(!client.supports(Model::Kling20));
    }

    /// Response delay exceeding the reqwest HTTP timeout (5s in `for_test`)
    /// must surface as `ProviderError::Timeout` so the router can fall back
    /// to a different provider instead of hanging the user.
    #[tokio::test]
    async fn response_delay_yields_timeout() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/fal-ai/flux-pro"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({
                        "images": [{ "url": "https://cdn/late.png", "width": 1, "height": 1 }]
                    }))
                    .set_delay(std::time::Duration::from_secs(10)),
            )
            .mount(&server)
            .await;

        let client = FalClient::for_test(key_store_with_key(), server.uri());
        let err = client
            .execute(Model::FalFluxPro, &request("hang please", Value::Null))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ProviderError::Timeout),
            "expected Timeout, got {err:?}"
        );
    }
}
