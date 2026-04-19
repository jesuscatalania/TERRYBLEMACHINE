//! Production [`DepthPipeline`] that dispatches through [`AiRouter`].
//!
//! Request shaping lives here: we build an [`AiRequest`] with
//! [`TaskKind::DepthMap`] so the router's default strategy picks
//! [`Model::ReplicateDepthAnythingV2`] and the Replicate client shapes the
//! body as `{ version, input: { image } }`.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::ai_router::{
    AiRequest, AiResponse, AiRouter, Complexity, Priority, RouterError, TaskKind,
};

use super::types::{DepthInput, DepthPipeline, DepthPipelineError, DepthResult};

pub struct RouterDepthPipeline {
    router: Arc<AiRouter>,
}

impl RouterDepthPipeline {
    pub fn new(router: Arc<AiRouter>) -> Self {
        Self { router }
    }
}

fn router_to_pipeline_err(err: RouterError) -> DepthPipelineError {
    DepthPipelineError::Router(err.to_string())
}

/// Extract the depth-map URL from the provider response. Replicate returns
/// the URL as the plain `output` string when `Prefer: wait` resolves the
/// prediction synchronously; fall back to a nested `depth_url` for
/// forward-compat with a future async-polling wrapper that might rename it.
fn first_depth_url(resp: &AiResponse) -> Option<String> {
    let output = &resp.output;
    if let Some(s) = output.get("depth_url").and_then(|v| v.as_str()) {
        return Some(s.to_string());
    }
    if let Some(s) = output.get("output").and_then(|v| v.as_str()) {
        return Some(s.to_string());
    }
    None
}

#[async_trait]
impl DepthPipeline for RouterDepthPipeline {
    async fn generate(&self, input: DepthInput) -> Result<DepthResult, DepthPipelineError> {
        let image_url = input.image_url.trim();
        if image_url.is_empty() {
            return Err(DepthPipelineError::InvalidInput(
                "image_url required".into(),
            ));
        }
        // Defense in depth: Replicate's depth-anything endpoint requires a
        // publicly-fetchable URL and cannot ingest data-URLs. Catch at the
        // pipeline boundary so a stale frontend caller gets a clear error
        // instead of an opaque Permanent failure deep in the provider.
        // Matches the image_pipeline inpaint guard (FU #103 pattern).
        if image_url.starts_with("data:") {
            return Err(DepthPipelineError::InvalidInput(
                "depth: hosted image URL required — data-URLs unsupported".into(),
            ));
        }

        let req = AiRequest {
            id: uuid::Uuid::new_v4().to_string(),
            task: TaskKind::DepthMap,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: String::new(),
            payload: json!({ "image_url": image_url }),
            model_override: None,
        };

        let resp = self
            .router
            .route(req)
            .await
            .map_err(router_to_pipeline_err)?;

        let depth_url = first_depth_url(&resp).ok_or(DepthPipelineError::NoOutput)?;
        if depth_url.is_empty() {
            return Err(DepthPipelineError::NoOutput);
        }
        Ok(DepthResult {
            depth_url,
            model: format!("{:?}", resp.model),
            cached: resp.cached,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Data-URL guard fires before any routing so we can verify it with a
    /// zero-client router. Matches the image_pipeline inpaint test.
    #[tokio::test]
    async fn rejects_data_url_image() {
        use crate::ai_router::{DefaultRoutingStrategy, PriorityQueue, RetryPolicy};

        let router = Arc::new(AiRouter::new(
            Arc::new(DefaultRoutingStrategy),
            HashMap::new(),
            RetryPolicy::default_policy(),
            Arc::new(PriorityQueue::new()),
        ));
        let pipeline = RouterDepthPipeline::new(router);

        let err = pipeline
            .generate(DepthInput {
                image_url: "data:image/png;base64,iVBORw0KGgo=".into(),
                module: None,
            })
            .await
            .expect_err("data-URL must be rejected before routing");

        assert!(
            matches!(err, DepthPipelineError::InvalidInput(_)),
            "expected InvalidInput, got {err:?}"
        );
        assert!(
            format!("{err}").contains("data-URLs"),
            "error message should mention data-URLs, got: {err}"
        );
    }

    #[tokio::test]
    async fn rejects_empty_image_url() {
        use crate::ai_router::{DefaultRoutingStrategy, PriorityQueue, RetryPolicy};

        let router = Arc::new(AiRouter::new(
            Arc::new(DefaultRoutingStrategy),
            HashMap::new(),
            RetryPolicy::default_policy(),
            Arc::new(PriorityQueue::new()),
        ));
        let pipeline = RouterDepthPipeline::new(router);

        let err = pipeline
            .generate(DepthInput {
                image_url: "   ".into(),
                module: None,
            })
            .await
            .expect_err("empty URL must be rejected before routing");

        assert!(
            matches!(err, DepthPipelineError::InvalidInput(_)),
            "expected InvalidInput, got {err:?}"
        );
    }
}
