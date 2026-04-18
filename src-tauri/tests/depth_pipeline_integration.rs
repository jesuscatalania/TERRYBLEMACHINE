//! End-to-End tests for [`RouterDepthPipeline`].
//!
//! Mirrors `image_pipeline_integration.rs`: we build a full `AiRouter` with
//! a fake Replicate client that returns a canned
//! `{ "output": "https://fake.replicate/depth.png" }` payload and fire the
//! pipeline through it. This pins the Routing-Verdrahtung
//! (TaskKind::DepthMap → Model::ReplicateDepthAnythingV2 → Provider::Replicate
//! → DepthResult) end-to-end without touching the network.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use terryblemachine_lib::ai_router::{
    AiClient, AiRequest, AiResponse, AiRouter, DefaultRoutingStrategy, Model, PriorityQueue,
    Provider, ProviderError, ProviderUsage, RetryPolicy,
};
use terryblemachine_lib::depth_pipeline::{
    DepthInput, DepthPipeline, DepthPipelineError, RouterDepthPipeline,
};

/// Fake Replicate client that returns a depth-anything–shaped response:
/// `{ output: "https://..." }`, i.e. Replicate's standard flat-output for
/// single-URL models when `Prefer: wait` resolves the prediction inline.
struct StubReplicate;

#[async_trait]
impl AiClient for StubReplicate {
    fn provider(&self) -> Provider {
        Provider::Replicate
    }

    fn supports(&self, m: Model) -> bool {
        matches!(m, Model::ReplicateDepthAnythingV2 | Model::ReplicateFluxDev)
    }

    async fn execute(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({ "output": "https://fake.replicate/depth.png" }),
            cost_cents: None,
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

fn pipeline() -> RouterDepthPipeline {
    let mut clients: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();
    clients.insert(Provider::Replicate, Arc::new(StubReplicate));
    let router = Arc::new(AiRouter::new(
        Arc::new(DefaultRoutingStrategy),
        clients,
        RetryPolicy::default_policy(),
        Arc::new(PriorityQueue::new()),
    ));
    RouterDepthPipeline::new(router)
}

#[tokio::test]
async fn depth_generates_url_from_provider() {
    let p = pipeline();
    let r = p
        .generate(DepthInput {
            image_url: "https://src/a.png".into(),
            module: None,
        })
        .await
        .expect("depth generation succeeds");

    assert!(
        r.depth_url.contains("fake.replicate"),
        "expected stub depth URL, got {}",
        r.depth_url
    );
    assert!(r.depth_url.ends_with("depth.png"));
    assert_eq!(r.model, format!("{:?}", Model::ReplicateDepthAnythingV2));
    assert!(!r.cached);
}

#[tokio::test]
async fn depth_rejects_data_url() {
    let p = pipeline();
    let err = p
        .generate(DepthInput {
            image_url: "data:image/png;base64,xyz".into(),
            module: None,
        })
        .await
        .expect_err("data-URL source must be rejected before routing");

    assert!(
        matches!(err, DepthPipelineError::InvalidInput(_)),
        "expected InvalidInput, got {err:?}"
    );
    assert!(
        format!("{err}").contains("data-URLs"),
        "expected error message to mention data-URLs, got: {err}"
    );
}
