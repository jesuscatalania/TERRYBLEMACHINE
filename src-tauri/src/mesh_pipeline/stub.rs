//! Deterministic stub used by tests and as the default backend before a
//! Meshy API key is configured. Mirrors `StubDepthPipeline`.

use async_trait::async_trait;

use super::types::{MeshImageInput, MeshPipeline, MeshPipelineError, MeshResult, MeshTextInput};

#[derive(Default)]
pub struct StubMeshPipeline;

impl StubMeshPipeline {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MeshPipeline for StubMeshPipeline {
    async fn generate_from_text(
        &self,
        input: MeshTextInput,
    ) -> Result<MeshResult, MeshPipelineError> {
        if input.prompt.trim().is_empty() {
            return Err(MeshPipelineError::InvalidInput("prompt empty".into()));
        }
        Ok(MeshResult {
            glb_url: format!("stub://mesh/text/{}.glb", input.prompt.len()),
            local_path: None,
            model: "StubMeshyText".into(),
        })
    }

    async fn generate_from_image(
        &self,
        input: MeshImageInput,
    ) -> Result<MeshResult, MeshPipelineError> {
        if input.image_url.trim().is_empty() {
            return Err(MeshPipelineError::InvalidInput("image_url required".into()));
        }
        if input.image_url.starts_with("data:") {
            return Err(MeshPipelineError::InvalidInput(
                "data-URL unsupported".into(),
            ));
        }
        Ok(MeshResult {
            glb_url: format!("stub://mesh/image/{}.glb", input.image_url.len()),
            local_path: None,
            model: "StubMeshyImage".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn text_returns_deterministic_url() {
        let p = StubMeshPipeline::new();
        let a = p
            .generate_from_text(MeshTextInput {
                prompt: "a cup".into(),
                module: None,
            })
            .await
            .unwrap();
        let b = p
            .generate_from_text(MeshTextInput {
                prompt: "a cup".into(),
                module: None,
            })
            .await
            .unwrap();
        assert_eq!(a.glb_url, b.glb_url);
        assert_eq!(a.model, "StubMeshyText");
        assert!(a.local_path.is_none());
    }

    #[tokio::test]
    async fn text_rejects_empty_prompt() {
        let p = StubMeshPipeline::new();
        let err = p
            .generate_from_text(MeshTextInput {
                prompt: "   ".into(),
                module: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, MeshPipelineError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn image_rejects_data_url() {
        let p = StubMeshPipeline::new();
        let err = p
            .generate_from_image(MeshImageInput {
                image_url: "data:image/png;base64,xyz".into(),
                prompt: None,
                module: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, MeshPipelineError::InvalidInput(_)));
    }
}
