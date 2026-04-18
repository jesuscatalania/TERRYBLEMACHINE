//! Deterministic stub used by tests and as the default backend before a
//! Replicate API key is configured.

use async_trait::async_trait;

use super::types::{DepthInput, DepthPipeline, DepthPipelineError, DepthResult};

#[derive(Default)]
pub struct StubDepthPipeline;

impl StubDepthPipeline {
    pub fn new() -> Self {
        Self
    }
}

fn hash_url(url: &str) -> String {
    // Simple deterministic hash — sum of bytes mod 2^32, hex-encoded. Avoids
    // pulling in an extra crate just to make a stable filename.
    let n: u64 = url.bytes().map(|b| b as u64).sum();
    format!("{n:x}")
}

#[async_trait]
impl DepthPipeline for StubDepthPipeline {
    async fn generate(&self, input: DepthInput) -> Result<DepthResult, DepthPipelineError> {
        let image_url = input.image_url.trim();
        if image_url.is_empty() {
            return Err(DepthPipelineError::InvalidInput(
                "image_url required".into(),
            ));
        }
        if image_url.starts_with("data:") {
            return Err(DepthPipelineError::InvalidInput(
                "depth: hosted image URL required — data-URLs unsupported".into(),
            ));
        }
        Ok(DepthResult {
            depth_url: format!("stub://depth/{}.png", hash_url(image_url)),
            model: "StubDepth".into(),
            cached: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_deterministic_stub_url() {
        let p = StubDepthPipeline::new();
        let a = p
            .generate(DepthInput {
                image_url: "https://x/a.png".into(),
                module: None,
            })
            .await
            .unwrap();
        let b = p
            .generate(DepthInput {
                image_url: "https://x/a.png".into(),
                module: None,
            })
            .await
            .unwrap();
        assert_eq!(a.depth_url, b.depth_url);
        assert!(a.depth_url.starts_with("stub://depth/"));
        assert_eq!(a.model, "StubDepth");
    }

    #[tokio::test]
    async fn rejects_empty_url() {
        let p = StubDepthPipeline::new();
        let err = p
            .generate(DepthInput {
                image_url: "   ".into(),
                module: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DepthPipelineError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn rejects_data_url() {
        let p = StubDepthPipeline::new();
        let err = p
            .generate(DepthInput {
                image_url: "data:image/png;base64,xyz".into(),
                module: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DepthPipelineError::InvalidInput(_)));
    }
}
