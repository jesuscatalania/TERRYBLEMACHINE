//! End-to-End tests for [`RouterImagePipeline`].
//!
//! Diese Tests schließen die TDD-Lücke aus Plan 4.1 / Audit P4 #87. Wir
//! bauen einen kompletten `AiRouter` mit einem fake-fal-Client auf und
//! fahren alle vier Pipeline-Methoden (text_to_image, image_to_image,
//! upscale, variants) gegen ihn. Damit ist die Routing-Verdrahtung
//! (TaskKind → Model → Provider → ImageResult) vollständig abgedeckt.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::json;

use terryblemachine_lib::ai_router::{
    AiClient, AiRequest, AiResponse, AiRouter, Complexity, DefaultRoutingStrategy, Model,
    PriorityQueue, Provider, ProviderError, ProviderUsage, RetryPolicy,
};
use terryblemachine_lib::image_pipeline::{
    GenerateVariantsInput, Image2ImageInput, ImagePipeline, InpaintInput, RouterImagePipeline,
    Text2ImageInput, UpscaleInput,
};

/// Fake AiClient that always succeeds and returns a fal.ai-shaped payload
/// (`{ "images": [{ "url": ..., "width": ..., "height": ... }] }`) which
/// [`RouterImagePipeline`] parses via its `first_image_url` helper.
///
/// The client claims to be `Provider::Fal` and declares support for every
/// model, so we can register the same instance under both `Provider::Fal`
/// and `Provider::Replicate` and also satisfy the image-edit/upscale routes.
///
/// Captures the last [`AiRequest`] it saw so tests can assert that the
/// pipeline forwarded provider-specific payload fields (source URL, scale, …)
/// without reaching over the wire. Closes FU #93.
struct StubFalClient {
    captured: Arc<Mutex<Option<AiRequest>>>,
}

impl StubFalClient {
    fn new() -> Self {
        Self {
            captured: Arc::new(Mutex::new(None)),
        }
    }

    fn captured_handle(&self) -> Arc<Mutex<Option<AiRequest>>> {
        Arc::clone(&self.captured)
    }
}

#[async_trait]
impl AiClient for StubFalClient {
    fn provider(&self) -> Provider {
        Provider::Fal
    }

    fn supports(&self, _model: Model) -> bool {
        true
    }

    async fn execute(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        *self.captured.lock().expect("captured mutex poisoned") = Some(request.clone());
        let url = format!("https://fake.fal/{}.png", request.prompt.len());
        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({
                "images": [{
                    "url": url,
                    "width": 1024,
                    "height": 1024,
                }],
                "seed": 42,
            }),
            cost_cents: Some(0),
            cached: false,
        })
    }

    async fn health_check(&self) -> bool {
        true
    }

    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
        Ok(ProviderUsage::default())
    }
}

/// Harness bundling a pipeline and a handle to the stub's captured request,
/// so tests can assert payload passthrough after `await`.
struct PipelineHarness {
    pipeline: RouterImagePipeline,
    captured: Arc<Mutex<Option<AiRequest>>>,
}

impl PipelineHarness {
    fn last_request(&self) -> AiRequest {
        self.captured
            .lock()
            .expect("captured mutex poisoned")
            .clone()
            .expect("expected StubFalClient::execute to have been called")
    }
}

fn build_pipeline() -> RouterImagePipeline {
    build_harness().pipeline
}

fn build_harness() -> PipelineHarness {
    let stub = Arc::new(StubFalClient::new());
    let captured = stub.captured_handle();
    let stub_trait: Arc<dyn AiClient> = stub;
    let mut clients: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();
    clients.insert(Provider::Fal, Arc::clone(&stub_trait));
    // Same client registered for Replicate so ImageGeneration's fallback
    // chain (FalFluxPro → ReplicateFluxDev) resolves cleanly even if the
    // primary hand-off changes.
    clients.insert(Provider::Replicate, Arc::clone(&stub_trait));

    let router = Arc::new(AiRouter::new(
        Arc::new(DefaultRoutingStrategy),
        clients,
        RetryPolicy::default_policy(),
        Arc::new(PriorityQueue::new()),
    ));
    PipelineHarness {
        pipeline: RouterImagePipeline::new(router),
        captured,
    }
}

#[tokio::test]
async fn text_to_image_returns_fal_url() {
    let pipeline = build_pipeline();
    let result = pipeline
        .text_to_image(Text2ImageInput {
            prompt: "abc".into(),
            complexity: Complexity::Medium,
            module: "graphic2d".into(),
        })
        .await
        .expect("text_to_image succeeds");

    assert!(
        result.url.starts_with("https://fake.fal/"),
        "expected stub URL, got {}",
        result.url
    );
    assert!(result.url.ends_with(".png"));
    // ImageGeneration + Medium routes to FalFluxPro via the default strategy.
    assert_eq!(result.model, format!("{:?}", Model::FalFluxPro));
    assert_eq!(result.width, Some(1024));
    assert_eq!(result.height, Some(1024));
}

#[tokio::test]
async fn image_to_image_passes_source_url() {
    let harness = build_harness();
    let source = "https://src/a.png";
    let result = harness
        .pipeline
        .image_to_image(Image2ImageInput {
            prompt: "make it neon".into(),
            image_url: source.into(),
            complexity: Complexity::Medium,
            module: "graphic2d".into(),
        })
        .await
        .expect("image_to_image succeeds");

    assert!(!result.url.is_empty());
    assert!(result.url.starts_with("https://fake.fal/"));
    // ImageEdit routes to FalFluxPro.
    assert_eq!(result.model, format!("{:?}", Model::FalFluxPro));

    // Assert the source URL actually reached the AiClient via the payload —
    // proves the pipeline serializes image_url into the JSON body and the
    // router doesn't strip it. (FU #93)
    let captured = harness.last_request();
    assert_eq!(
        captured.payload.get("image_url").and_then(|v| v.as_str()),
        Some(source),
        "expected source image_url to reach the client unchanged, got payload: {}",
        captured.payload
    );
}

#[tokio::test]
async fn upscale_returns_result() {
    let harness = build_harness();
    let result = harness
        .pipeline
        .upscale(UpscaleInput {
            image_url: "file:///tmp/small.png".into(),
            scale: 2,
        })
        .await
        .expect("upscale succeeds");

    assert!(!result.url.is_empty());
    // Upscale routes to FalRealEsrgan.
    assert_eq!(result.model, format!("{:?}", Model::FalRealEsrgan));

    // Assert the scale factor actually reached the AiClient via the payload —
    // the provider wiring would silently drop an unsupported parameter name
    // if we refactored the caller without this check. (FU #93)
    let captured = harness.last_request();
    assert_eq!(
        captured.payload.get("scale").and_then(|v| v.as_u64()),
        Some(2),
        "expected scale=2 to reach the client unchanged, got payload: {}",
        captured.payload
    );
    assert_eq!(
        captured.payload.get("image_url").and_then(|v| v.as_str()),
        Some("file:///tmp/small.png"),
        "expected upscale image_url to reach the client, got payload: {}",
        captured.payload
    );
}

#[tokio::test]
async fn inpaint_routes_to_fal_flux_fill() {
    let pipeline = build_pipeline();
    let result = pipeline
        .inpaint(InpaintInput {
            prompt: "replace with flowers".into(),
            source_url: "https://fake.fal/src.png".into(),
            mask_url: "https://fake.fal/mask.png".into(),
            complexity: Complexity::Medium,
            module: "graphic2d".into(),
        })
        .await
        .expect("inpaint succeeds");

    assert!(result.url.starts_with("https://fake.fal/"));
    // Inpaint is hard-wired to FalFluxFill in the default strategy.
    assert_eq!(result.model, format!("{:?}", Model::FalFluxFill));
}

#[tokio::test]
async fn inpaint_rejects_empty_prompt() {
    let pipeline = build_pipeline();
    let err = pipeline
        .inpaint(InpaintInput {
            prompt: "  ".into(),
            source_url: "https://fake.fal/src.png".into(),
            mask_url: "https://fake.fal/mask.png".into(),
            complexity: Complexity::Medium,
            module: "graphic2d".into(),
        })
        .await
        .expect_err("empty prompt must be rejected");
    assert!(
        format!("{err:?}").contains("InvalidInput"),
        "expected InvalidInput, got {err:?}"
    );
}

#[tokio::test]
async fn inpaint_rejects_data_urls() {
    let pipeline = build_pipeline();
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
        format!("{err:?}").contains("InvalidInput"),
        "expected InvalidInput for data-URL source, got {err:?}"
    );
    assert!(
        format!("{err}").contains("data-URLs"),
        "expected error message to mention data-URLs, got: {err}"
    );
}

#[tokio::test]
async fn variants_yields_requested_count() {
    let pipeline = build_pipeline();
    let results = pipeline
        .variants(GenerateVariantsInput {
            prompt: "logo draft".into(),
            count: 4,
            complexity: Complexity::Medium,
            module: "graphic2d".into(),
        })
        .await
        .expect("variants succeeds");

    // The router caches identical (prompt, model, payload) triples, so the
    // pipeline salts each variant with a random `variant_seed` in the
    // payload — we should get exactly `count` results back.
    assert_eq!(results.len(), 4);
    for r in &results {
        assert!(r.url.starts_with("https://fake.fal/"));
        assert_eq!(r.model, format!("{:?}", Model::FalFluxPro));
    }
}
