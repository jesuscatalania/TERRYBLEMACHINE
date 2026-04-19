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
    AiClient, AiRequest, AiResponse, Model, Provider, ProviderError, ProviderUsage, TaskKind,
};
use crate::keychain::KeyStore;

/// Default base URL for the fal.ai queue endpoints — used by Kling (long
/// async video renders must submit + poll). Returns `{request_id,
/// status_url, response_url}`.
pub const DEFAULT_BASE_URL: &str = "https://queue.fal.run";
/// Default base URL for fal.ai's synchronous proxy — used by the image
/// models (Flux Pro, SDXL, Real-ESRGAN, Flux-Fill). The proxy internally
/// runs the same queue but blocks the HTTP connection until the job
/// finishes, returning the result inline (same shape as the legacy sync
/// API). Keeps the existing `send_request` parse path valid.
pub const DEFAULT_SYNC_BASE_URL: &str = "https://fal.run";
/// Keychain service id under which the fal.ai key lives.
pub const KEYCHAIN_SERVICE: &str = "fal";
/// Default rate: fal.ai free tier is lenient; 10 rps is a safe ceiling.
const DEFAULT_RATE_PER_SEC: usize = 10;

/// Upper bound on queue-status polling for Kling-via-fal video renders.
/// Exponential back-off from 3s→15s gives ~5 minutes of render head-room;
/// fal's Kling typically completes inside 60–120s but can drift when
/// upstream capacity is scarce.
const DEFAULT_POLL_MAX_ATTEMPTS: u32 = 60;
const DEFAULT_POLL_INITIAL_DELAY: Duration = Duration::from_secs(3);
const DEFAULT_POLL_MAX_DELAY: Duration = Duration::from_secs(15);

pub struct FalClient {
    http: Client,
    /// Queue base URL — used by Kling (async submit + poll).
    base_url: String,
    /// Sync-proxy base URL — used by image models (blocking POST).
    sync_base_url: String,
    key_store: Arc<dyn KeyStore>,
    rate: RateLimiter,
    poll_max_attempts: u32,
    poll_initial_delay: Duration,
    poll_max_delay: Duration,
}

impl FalClient {
    pub fn new(key_store: Arc<dyn KeyStore>) -> Self {
        Self::with_base_urls(
            key_store,
            DEFAULT_BASE_URL.to_owned(),
            DEFAULT_SYNC_BASE_URL.to_owned(),
            DEFAULT_RATE_PER_SEC,
        )
    }

    pub fn with_base_url(
        key_store: Arc<dyn KeyStore>,
        base_url: String,
        rate_per_sec: usize,
    ) -> Self {
        let sync = base_url.clone();
        Self::with_base_urls(key_store, base_url, sync, rate_per_sec)
    }

    pub fn with_base_urls(
        key_store: Arc<dyn KeyStore>,
        base_url: String,
        sync_base_url: String,
        rate_per_sec: usize,
    ) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("reqwest client builds");
        Self {
            http,
            base_url,
            sync_base_url,
            key_store,
            rate: RateLimiter::new(rate_per_sec),
            poll_max_attempts: DEFAULT_POLL_MAX_ATTEMPTS,
            poll_initial_delay: DEFAULT_POLL_INITIAL_DELAY,
            poll_max_delay: DEFAULT_POLL_MAX_DELAY,
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
            sync_base_url: base_url.clone(),
            base_url,
            key_store,
            rate: RateLimiter::unlimited(),
            // Tests should not sit spinning for minutes; shrink every knob.
            poll_max_attempts: DEFAULT_POLL_MAX_ATTEMPTS,
            poll_initial_delay: Duration::from_millis(10),
            poll_max_delay: Duration::from_millis(100),
        }
    }

    /// Test-friendly variant that accepts a custom HTTP timeout. Used by the
    /// timeout-pillar wiremock tests to keep wall-clock cost low (single-digit
    /// ms instead of waiting on the default 5s timeout).
    #[cfg(test)]
    pub fn for_test_with_http_timeout(
        key_store: Arc<dyn KeyStore>,
        base_url: String,
        http_timeout: Duration,
    ) -> Self {
        let http = Client::builder()
            .timeout(http_timeout)
            .build()
            .expect("reqwest client builds");
        Self {
            http,
            sync_base_url: base_url.clone(),
            base_url,
            key_store,
            rate: RateLimiter::unlimited(),
            poll_max_attempts: DEFAULT_POLL_MAX_ATTEMPTS,
            poll_initial_delay: Duration::from_millis(10),
            poll_max_delay: Duration::from_millis(100),
        }
    }

    /// Like [`Self::for_test`], but with a custom cap on poll attempts. Used
    /// by the Kling-via-fal polling tests to keep wall-clock cost low when
    /// the queue never reaches a terminal state.
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

    /// The endpoint path for a given supported model.
    //
    // Naming note: the `Model::FalFluxPro` variant maps to the fal.ai
    // endpoint for Flux Pro (marketed as "Flux 1.1 Pro"). The plan doc
    // uses "Flux 2 Pro" as an aspirational name; when fal.ai ships a v2
    // endpoint, add the new `Model` variant and a new arm here.
    //
    // Kling-via-fal models route through this method too but the leaf
    // segment (`/text-to-video` vs `/image-to-video`) depends on the
    // [`TaskKind`], so callers use [`Self::kling_endpoint_for`] instead.
    fn endpoint_for(model: Model) -> Option<&'static str> {
        match model {
            Model::FalFluxPro => Some("/fal-ai/flux-pro"),
            Model::FalSdxl => Some("/fal-ai/fast-sdxl"),
            Model::FalRealEsrgan => Some("/fal-ai/real-esrgan"),
            Model::FalFluxFill => Some("/fal-ai/flux-fill"),
            _ => None,
        }
    }

    /// Kling-via-fal endpoint for a given (model, task) pair. Returns
    /// `None` when the model isn't a fal-Kling variant or the task isn't
    /// text-to-video / image-to-video.
    fn kling_endpoint_for(model: Model, task: TaskKind) -> Option<&'static str> {
        match (model, task) {
            (Model::FalKlingV15, TaskKind::TextToVideo) => {
                Some("/fal-ai/kling-video/v1.5/standard/text-to-video")
            }
            (Model::FalKlingV15, TaskKind::ImageToVideo) => {
                Some("/fal-ai/kling-video/v1.5/standard/image-to-video")
            }
            (Model::FalKlingV2Master, TaskKind::TextToVideo) => {
                Some("/fal-ai/kling-video/v2/master/text-to-video")
            }
            (Model::FalKlingV2Master, TaskKind::ImageToVideo) => {
                Some("/fal-ai/kling-video/v2/master/image-to-video")
            }
            _ => None,
        }
    }

    /// Whether `model` is one of the Kling-via-fal aggregator endpoints
    /// (`send_request` routes those through the queue/poll flow).
    fn is_fal_kling(model: Model) -> bool {
        matches!(model, Model::FalKlingV15 | Model::FalKlingV2Master)
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
            Model::FalKlingV15 | Model::FalKlingV2Master => {
                // fal's Kling endpoints expect `duration` as a STRING (e.g.
                // "5" / "10"). Default 5s matches the legacy direct-Kling
                // client behaviour (`DEFAULT_DURATION_SEC`).
                let duration = request
                    .payload
                    .get("duration")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or(5);
                let aspect = request
                    .payload
                    .get("aspect_ratio")
                    .and_then(|v| v.as_str())
                    .unwrap_or("16:9");
                let mut body = json!({
                    "prompt": request.prompt,
                    "duration": duration.to_string(),
                    "aspect_ratio": aspect,
                });
                if request.task == TaskKind::ImageToVideo {
                    let image_url = request
                        .payload
                        .get("image_url")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ProviderError::Permanent(
                                "fal kling image-to-video: image_url required".into(),
                            )
                        })?;
                    body.as_object_mut()
                        .expect("json!({..}) yields an object")
                        .insert("image_url".into(), json!(image_url));
                }
                Ok(body)
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
        if Self::is_fal_kling(model) {
            return self.send_kling_request(model, request).await;
        }

        self.rate.acquire().await;
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;
        let endpoint = Self::endpoint_for(model)
            .ok_or_else(|| ProviderError::Permanent(format!("unsupported model {model:?}")))?;
        let body = Self::body_for(model, request)?;

        // Image models route through the synchronous-proxy host; Kling
        // (long-running video) uses the queue host via send_kling_request.
        let url = format!("{}{}", self.sync_base_url, endpoint);
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
            _ => unreachable!("endpoint_for + is_fal_kling partition the supported model set"),
        };

        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output,
            cost_cents: None,
            cached: false,
        })
    }

    /// Submit a Kling-video request through fal.ai's queue API, poll the
    /// status URL until COMPLETED, then fetch the result and extract
    /// `video.url`. Returns an [`AiResponse`] whose `output.video_url`
    /// matches the shape produced by the direct-Kling client — so the
    /// video pipeline consumes both transparently.
    async fn send_kling_request(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        self.rate.acquire().await;
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;
        let endpoint = Self::kling_endpoint_for(model, request.task).ok_or_else(|| {
            ProviderError::Permanent(format!(
                "fal kling: unsupported (model, task) pair: {:?}/{:?}",
                model, request.task,
            ))
        })?;
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

        let submit: Value = resp.json().await.map_err(map_reqwest_error)?;
        let request_id = submit
            .get("request_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let status_url = submit
            .get("status_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Permanent("fal kling submit: missing status_url".into()))?
            .to_string();
        let response_url = submit
            .get("response_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProviderError::Permanent("fal kling submit: missing response_url".into())
            })?
            .to_string();

        let final_body = self.poll_request(&status_url, &response_url).await?;

        // fal-Kling result body shape: `{ "video": { "url": ... }, "seed": .. }`.
        let video_url = final_body
            .get("video")
            .and_then(|v| v.get("url"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProviderError::Permanent("fal kling: COMPLETED response missing video.url".into())
            })?
            .to_string();

        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({
                "job_id": request_id,
                "status": "succeeded",
                "video_url": video_url,
            }),
            cost_cents: None,
            cached: false,
        })
    }

    /// Poll fal.ai's queue status endpoint until terminal, then fetch and
    /// return the result body. Mirrors
    /// [`super::kling::KlingClient::poll_task`]: `COMPLETED` → fetch +
    /// return result body; `FAILED` → [`ProviderError::Permanent`];
    /// exhaustion → [`ProviderError::JobAlreadySubmitted`] so the router
    /// does NOT re-POST (creates a duplicate billable job).
    async fn poll_request(
        &self,
        status_url: &str,
        response_url: &str,
    ) -> Result<Value, ProviderError> {
        let key = get_api_key(&*self.key_store, KEYCHAIN_SERVICE)?;
        let mut delay = self.poll_initial_delay;

        for _ in 0..self.poll_max_attempts {
            self.rate.acquire().await;
            let resp = self
                .http
                .get(status_url)
                .header("authorization", format!("Key {key}"))
                .send()
                .await
                .map_err(map_reqwest_error)?;

            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(map_http_error(status, &text));
            }
            let body: Value = resp.json().await.map_err(map_reqwest_error)?;
            let s = body
                .get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_ascii_uppercase());

            match s.as_deref() {
                Some("COMPLETED") => {
                    let result = self
                        .http
                        .get(response_url)
                        .header("authorization", format!("Key {key}"))
                        .send()
                        .await
                        .map_err(map_reqwest_error)?;
                    let result_status = result.status();
                    if !result_status.is_success() {
                        let text = result.text().await.unwrap_or_default();
                        return Err(map_http_error(result_status, &text));
                    }
                    return result.json().await.map_err(map_reqwest_error);
                }
                Some("FAILED") => {
                    let msg = body
                        .get("error")
                        .and_then(|v| v.as_str())
                        .or_else(|| body.get("message").and_then(|v| v.as_str()))
                        .unwrap_or("fal-ai task failed")
                        .to_string();
                    return Err(ProviderError::Permanent(msg));
                }
                // IN_QUEUE / IN_PROGRESS / unknown — keep polling.
                _ => {
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(self.poll_max_delay);
                }
            }
        }

        Err(ProviderError::JobAlreadySubmitted(format!(
            "fal queue request did not reach a terminal status within {} poll attempts",
            self.poll_max_attempts
        )))
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
        Self::endpoint_for(model).is_some() || Self::is_fal_kling(model)
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
    use wiremock::matchers::{body_partial_json, header, method, path};
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
            model_override: None,
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
        // Kling-via-fal — routed through the same client / key.
        assert!(client.supports(Model::FalKlingV15));
        assert!(client.supports(Model::FalKlingV2Master));
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
                    .set_delay(std::time::Duration::from_millis(100)),
            )
            .mount(&server)
            .await;

        let client = FalClient::for_test_with_http_timeout(
            key_store_with_key(),
            server.uri(),
            Duration::from_millis(50),
        );
        let err = client
            .execute(Model::FalFluxPro, &request("hang please", Value::Null))
            .await
            .unwrap_err();
        assert!(
            matches!(err, ProviderError::Timeout),
            "expected Timeout, got {err:?}"
        );
    }

    // ─── Kling-via-fal (queue + poll) ─────────────────────────────────
    //
    // Both tests drive the full `POST /submit` → `GET /status` (COMPLETED)
    // → `GET /result` flow and assert the response shape matches what the
    // video pipeline expects (`output.video_url` as a string, to mirror
    // the direct-Kling client contract).

    #[tokio::test]
    async fn kling_v2_master_text_to_video_polls_then_returns_url() {
        let server = MockServer::start().await;
        // fal's queue API returns absolute URLs for status/response — the
        // mock points them back at this MockServer so polling stays local.
        let status_url = format!("{}/fal-ai/kling-video/requests/req-v2/status", server.uri());
        let response_url = format!("{}/fal-ai/kling-video/requests/req-v2", server.uri());

        Mock::given(method("POST"))
            .and(path("/fal-ai/kling-video/v2/master/text-to-video"))
            .and(header("authorization", "Key fal-test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "request_id": "req-v2",
                "status_url": status_url,
                "response_url": response_url,
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/fal-ai/kling-video/requests/req-v2/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "COMPLETED",
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/fal-ai/kling-video/requests/req-v2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "video": {
                    "url": "https://storage.fal.media/files/abc/video.mp4",
                    "content_type": "video/mp4",
                    "file_size": 1234567_u64,
                },
                "seed": 42,
            })))
            .mount(&server)
            .await;

        let client = FalClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "fal-kling-v2-1".into(),
            task: TaskKind::TextToVideo,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: "a cat dancing".into(),
            payload: Value::Null,
            model_override: None,
        };
        let resp = client.execute(Model::FalKlingV2Master, &req).await.unwrap();
        assert_eq!(resp.model, Model::FalKlingV2Master);
        assert_eq!(
            resp.output.get("video_url").and_then(|v| v.as_str()),
            Some("https://storage.fal.media/files/abc/video.mp4")
        );
        assert_eq!(
            resp.output.get("job_id").and_then(|v| v.as_str()),
            Some("req-v2")
        );
        assert_eq!(
            resp.output.get("status").and_then(|v| v.as_str()),
            Some("succeeded")
        );
    }

    #[tokio::test]
    async fn kling_v15_image_to_video_includes_image_url() {
        let server = MockServer::start().await;
        let status_url = format!(
            "{}/fal-ai/kling-video/requests/req-v15/status",
            server.uri()
        );
        let response_url = format!("{}/fal-ai/kling-video/requests/req-v15", server.uri());

        Mock::given(method("POST"))
            .and(path("/fal-ai/kling-video/v1.5/standard/image-to-video"))
            .and(body_partial_json(json!({
                "image_url": "https://src/hero.png",
                "prompt": "cinematic dolly",
                "duration": "5",
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "request_id": "req-v15",
                "status_url": status_url,
                "response_url": response_url,
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/fal-ai/kling-video/requests/req-v15/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "COMPLETED",
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/fal-ai/kling-video/requests/req-v15"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "video": { "url": "https://storage.fal.media/files/xyz/i2v.mp4" }
            })))
            .mount(&server)
            .await;

        let client = FalClient::for_test(key_store_with_key(), server.uri());
        let req = AiRequest {
            id: "fal-kling-v15-1".into(),
            task: TaskKind::ImageToVideo,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: "cinematic dolly".into(),
            payload: json!({ "image_url": "https://src/hero.png" }),
            model_override: None,
        };
        let resp = client.execute(Model::FalKlingV15, &req).await.unwrap();
        assert_eq!(
            resp.output.get("video_url").and_then(|v| v.as_str()),
            Some("https://storage.fal.media/files/xyz/i2v.mp4")
        );
    }
}
