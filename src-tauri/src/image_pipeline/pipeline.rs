//! Production [`ImagePipeline`] that dispatches through [`AiRouter`].
//!
//! Request shaping lives here: we build [`AiRequest`]s with the correct
//! [`TaskKind`] so the router's default strategy picks fal.ai Flux Pro
//! for text-to-image, Replicate as fallback, and Real-ESRGAN for upscale.

use std::sync::Arc;

use async_trait::async_trait;
use futures::future::join_all;
use serde_json::json;

use crate::ai_router::{
    AiRequest, AiResponse, AiRouter, Complexity, Priority, RouterError, TaskKind,
};
use crate::taste_engine::{EnrichOptions, TasteEngine};

use super::types::{
    GenerateVariantsInput, Image2ImageInput, ImagePipeline, ImagePipelineError, ImageResult,
    InpaintInput, Text2ImageInput, UpscaleInput,
};

pub struct RouterImagePipeline {
    router: Arc<AiRouter>,
    taste: Option<Arc<TasteEngine>>,
}

impl RouterImagePipeline {
    pub fn new(router: Arc<AiRouter>) -> Self {
        Self {
            router,
            taste: None,
        }
    }

    pub fn with_taste_engine(mut self, engine: Arc<TasteEngine>) -> Self {
        self.taste = Some(engine);
        self
    }

    async fn enrich(&self, prompt: &str, module: &str) -> String {
        let Some(engine) = self.taste.as_ref() else {
            return prompt.to_string();
        };
        engine
            .enrich(
                prompt,
                &EnrichOptions {
                    module: Some(module.to_string()),
                    tags: Vec::new(),
                    with_negative: true,
                },
            )
            .await
    }

    async fn dispatch(&self, req: AiRequest) -> Result<AiResponse, ImagePipelineError> {
        self.router.route(req).await.map_err(router_to_pipeline_err)
    }
}

fn router_to_pipeline_err(err: RouterError) -> ImagePipelineError {
    ImagePipelineError::Router(err.to_string())
}

fn new_request(
    task: TaskKind,
    complexity: Complexity,
    prompt: String,
    payload: serde_json::Value,
) -> AiRequest {
    AiRequest {
        id: uuid::Uuid::new_v4().to_string(),
        task,
        priority: Priority::Normal,
        complexity,
        prompt,
        payload,
        model_override: None,
    }
}

/// Extract an image URL from a provider response. Every client we wired in
/// Schritt 2.2 encodes the URL under `images[0].url` or `image.url`.
fn first_image_url(resp: &AiResponse) -> Option<(String, Option<u32>, Option<u32>)> {
    let output = &resp.output;

    if let Some(arr) = output.get("images").and_then(|v| v.as_array()) {
        if let Some(first) = arr.first() {
            return Some((
                first
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                first
                    .get("width")
                    .and_then(|v| v.as_u64())
                    .map(|w| w as u32),
                first
                    .get("height")
                    .and_then(|v| v.as_u64())
                    .map(|h| h as u32),
            ));
        }
    }
    if let Some(obj) = output.get("image").and_then(|v| v.as_object()) {
        return Some((
            obj.get("url").and_then(|v| v.as_str()).unwrap_or("").into(),
            obj.get("width").and_then(|v| v.as_u64()).map(|w| w as u32),
            obj.get("height").and_then(|v| v.as_u64()).map(|h| h as u32),
        ));
    }
    None
}

fn response_to_result(resp: AiResponse) -> Result<ImageResult, ImagePipelineError> {
    let (url, w, h) = first_image_url(&resp).ok_or(ImagePipelineError::EmptyResponse)?;
    if url.is_empty() {
        return Err(ImagePipelineError::EmptyResponse);
    }
    Ok(ImageResult {
        url,
        width: w,
        height: h,
        seed: resp.output.get("seed").and_then(|v| v.as_u64()),
        model: format!("{:?}", resp.model),
        cached: resp.cached,
    })
}

#[async_trait]
impl ImagePipeline for RouterImagePipeline {
    async fn text_to_image(
        &self,
        input: Text2ImageInput,
    ) -> Result<ImageResult, ImagePipelineError> {
        if input.prompt.trim().is_empty() {
            return Err(ImagePipelineError::InvalidInput("empty prompt".into()));
        }
        let prompt = self.enrich(&input.prompt, &input.module).await;
        let mut req = new_request(
            TaskKind::ImageGeneration,
            input.complexity,
            prompt,
            json!({}),
        );
        req.model_override = input.model_override;
        let resp = self.dispatch(req).await?;
        response_to_result(resp)
    }

    async fn image_to_image(
        &self,
        input: Image2ImageInput,
    ) -> Result<ImageResult, ImagePipelineError> {
        if input.prompt.trim().is_empty() || input.image_url.trim().is_empty() {
            return Err(ImagePipelineError::InvalidInput(
                "prompt and image_url required".into(),
            ));
        }
        let prompt = self.enrich(&input.prompt, &input.module).await;
        let req = new_request(
            TaskKind::ImageEdit,
            input.complexity,
            prompt,
            json!({ "image_url": input.image_url }),
        );
        let resp = self.dispatch(req).await?;
        response_to_result(resp)
    }

    async fn upscale(&self, input: UpscaleInput) -> Result<ImageResult, ImagePipelineError> {
        if input.image_url.trim().is_empty() {
            return Err(ImagePipelineError::InvalidInput(
                "image_url required".into(),
            ));
        }
        let req = new_request(
            TaskKind::Upscale,
            Complexity::Simple,
            format!("upscale x{}", input.scale),
            json!({ "image_url": input.image_url, "scale": input.scale }),
        );
        let resp = self.dispatch(req).await?;
        response_to_result(resp)
    }

    async fn variants(
        &self,
        input: GenerateVariantsInput,
    ) -> Result<Vec<ImageResult>, ImagePipelineError> {
        if input.prompt.trim().is_empty() {
            return Err(ImagePipelineError::InvalidInput("empty prompt".into()));
        }
        let n = input.count.clamp(1, 8) as usize;
        let enriched = self.enrich(&input.prompt, &input.module).await;
        // `Model` is `Copy`, so capturing once and reassigning per-iteration
        // avoids any Arc/Clone dance while keeping every spawned request
        // independent.
        let override_capture = input.model_override;

        let mut futures = Vec::with_capacity(n);
        for _ in 0..n {
            let router = self.router.clone();
            let prompt = enriched.clone();
            let complexity = input.complexity;
            let override_clone = override_capture;
            futures.push(async move {
                let mut req = new_request(
                    TaskKind::ImageGeneration,
                    complexity,
                    prompt,
                    json!({ "variant_seed": uuid::Uuid::new_v4().to_string() }),
                );
                req.model_override = override_clone;
                router.route(req).await
            });
        }

        let results = join_all(futures).await;
        let mut out = Vec::new();
        for r in results {
            match r {
                Ok(resp) => {
                    if let Ok(image) = response_to_result(resp) {
                        out.push(image);
                    }
                }
                Err(_) => continue,
            }
        }
        if out.is_empty() {
            return Err(ImagePipelineError::AllVariantsFailed(n as u32));
        }
        Ok(out)
    }

    async fn inpaint(&self, input: InpaintInput) -> Result<ImageResult, ImagePipelineError> {
        // TODO(phase-5): KNOWN LIMITATION — fal.ai flux-fill requires
        // publicly-hosted URLs for both `image_url` and `mask_url`. Data-URLs
        // produced by the frontend canvas will fail at the provider layer
        // with a Permanent error. A local upload shim (serving data-URLs via
        // a short-lived localhost HTTP server, or uploading to fal.ai's
        // temporary storage endpoint) is planned for Phase 5. Until then,
        // the frontend should guard against data-URL inputs and surface a
        // clear error to the user.
        if input.prompt.trim().is_empty() {
            return Err(ImagePipelineError::InvalidInput("empty prompt".into()));
        }
        if input.source_url.trim().is_empty() || input.mask_url.trim().is_empty() {
            return Err(ImagePipelineError::InvalidInput(
                "source_url and mask_url required".into(),
            ));
        }
        // Defense in depth: fal.ai flux-fill cannot ingest data-URLs
        // (>>2MB payloads, no public reachability). The frontend already
        // guards, but catching this at the router boundary means a stale
        // or misconfigured caller gets a clear error instead of an opaque
        // Permanent failure deep inside the provider response.
        if input.source_url.starts_with("data:") || input.mask_url.starts_with("data:") {
            return Err(ImagePipelineError::InvalidInput(
                "inpaint: hosted URLs required — data-URLs unsupported (see Phase 5 upload pipeline)"
                    .into(),
            ));
        }
        let prompt = self.enrich(&input.prompt, &input.module).await;
        let req = new_request(
            TaskKind::Inpaint,
            input.complexity,
            prompt,
            json!({
                "image_url": input.source_url,
                "mask_url": input.mask_url,
            }),
        );
        let resp = self.dispatch(req).await?;
        response_to_result(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The data-URL guard must fire *before* any `AiRouter::route` call, so
    /// we can verify it by constructing the pipeline with a `None` router
    /// candidate: we use a dummy `AiRouter` built from an empty client map
    /// and assert the error surfaces as `InvalidInput` without needing any
    /// provider wiring. The guard is a pure string check, so even a
    /// zero-client router would fail *later* with `Router(...)` if we
    /// regressed — hence the explicit variant match.
    #[tokio::test]
    async fn inpaint_rejects_data_url_source() {
        use crate::ai_router::{AiRouter, DefaultRoutingStrategy, PriorityQueue, RetryPolicy};
        use std::collections::HashMap;

        let router = Arc::new(AiRouter::new(
            Arc::new(DefaultRoutingStrategy),
            HashMap::new(),
            RetryPolicy::default_policy(),
            Arc::new(PriorityQueue::new()),
        ));
        let pipeline = RouterImagePipeline::new(router);

        let err = pipeline
            .inpaint(InpaintInput {
                prompt: "replace with flowers".into(),
                source_url: "data:image/png;base64,iVBORw0KGgo=".into(),
                mask_url: "https://fake.fal/mask.png".into(),
                complexity: Complexity::Medium,
                module: "graphic2d".into(),
            })
            .await
            .expect_err("data-URL source must be rejected before routing");

        assert!(
            matches!(err, ImagePipelineError::InvalidInput(_)),
            "expected InvalidInput, got {err:?}"
        );
        assert!(
            format!("{err}").contains("data-URLs"),
            "error message should mention data-URLs, got: {err}"
        );
    }

    #[tokio::test]
    async fn inpaint_rejects_data_url_mask() {
        use crate::ai_router::{AiRouter, DefaultRoutingStrategy, PriorityQueue, RetryPolicy};
        use std::collections::HashMap;

        let router = Arc::new(AiRouter::new(
            Arc::new(DefaultRoutingStrategy),
            HashMap::new(),
            RetryPolicy::default_policy(),
            Arc::new(PriorityQueue::new()),
        ));
        let pipeline = RouterImagePipeline::new(router);

        let err = pipeline
            .inpaint(InpaintInput {
                prompt: "replace with flowers".into(),
                source_url: "https://fake.fal/src.png".into(),
                mask_url: "data:image/png;base64,iVBORw0KGgo=".into(),
                complexity: Complexity::Medium,
                module: "graphic2d".into(),
            })
            .await
            .expect_err("data-URL mask must be rejected before routing");

        assert!(
            matches!(err, ImagePipelineError::InvalidInput(_)),
            "expected InvalidInput, got {err:?}"
        );
    }
}
