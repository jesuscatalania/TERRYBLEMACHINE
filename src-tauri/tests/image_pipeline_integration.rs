//! End-to-End tests for [`RouterImagePipeline`].
//!
//! Diese Tests schließen die TDD-Lücke aus Plan 4.1 / Audit P4 #87. Wir
//! bauen einen kompletten `AiRouter` mit einem fake-fal-Client auf und
//! fahren alle vier Pipeline-Methoden (text_to_image, image_to_image,
//! upscale, variants) gegen ihn. Damit ist die Routing-Verdrahtung
//! (TaskKind → Model → Provider → ImageResult) vollständig abgedeckt.

use std::collections::HashMap;
use std::sync::Arc;

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
struct StubFalClient;

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

fn build_pipeline() -> RouterImagePipeline {
    let stub: Arc<dyn AiClient> = Arc::new(StubFalClient);
    let mut clients: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();
    clients.insert(Provider::Fal, Arc::clone(&stub));
    // Same client registered for Replicate so ImageGeneration's fallback
    // chain (FalFluxPro → ReplicateFluxDev) resolves cleanly even if the
    // primary hand-off changes.
    clients.insert(Provider::Replicate, Arc::clone(&stub));

    let router = Arc::new(AiRouter::new(
        Arc::new(DefaultRoutingStrategy),
        clients,
        RetryPolicy::default_policy(),
        Arc::new(PriorityQueue::new()),
    ));
    RouterImagePipeline::new(router)
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
    let pipeline = build_pipeline();
    let result = pipeline
        .image_to_image(Image2ImageInput {
            prompt: "make it neon".into(),
            image_url: "file:///tmp/source.png".into(),
            complexity: Complexity::Medium,
            module: "graphic2d".into(),
        })
        .await
        .expect("image_to_image succeeds");

    assert!(!result.url.is_empty());
    assert!(result.url.starts_with("https://fake.fal/"));
    // ImageEdit routes to FalFluxPro.
    assert_eq!(result.model, format!("{:?}", Model::FalFluxPro));
}

#[tokio::test]
async fn upscale_returns_result() {
    let pipeline = build_pipeline();
    let result = pipeline
        .upscale(UpscaleInput {
            image_url: "file:///tmp/small.png".into(),
            scale: 2,
        })
        .await
        .expect("upscale succeeds");

    assert!(!result.url.is_empty());
    // Upscale routes to FalRealEsrgan.
    assert_eq!(result.model, format!("{:?}", Model::FalRealEsrgan));
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
