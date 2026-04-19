//! Deterministic stub used by tests and as the default backend before an
//! Ideogram API key is configured. Mirrors `StubMeshPipeline`.

use async_trait::async_trait;

use super::types::{LogoInput, LogoPipeline, LogoPipelineError, LogoVariant};

#[derive(Default)]
pub struct StubLogoPipeline;

impl StubLogoPipeline {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LogoPipeline for StubLogoPipeline {
    async fn generate_variants(
        &self,
        input: LogoInput,
    ) -> Result<Vec<LogoVariant>, LogoPipelineError> {
        if input.prompt.trim().is_empty() {
            return Err(LogoPipelineError::InvalidInput("prompt empty".into()));
        }
        let count = input.count.clamp(1, 10);
        Ok((0..count)
            .map(|i| LogoVariant {
                url: format!("stub://logo/{}/{}.png", input.prompt.len(), i),
                local_path: None,
                seed: Some(i),
                model: "StubIdeogram".into(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logo_pipeline::LogoStyle;

    #[tokio::test]
    async fn rejects_empty_prompt() {
        let p = StubLogoPipeline::new();
        let err = p
            .generate_variants(LogoInput {
                prompt: "   ".into(),
                style: LogoStyle::Minimalist,
                count: 3,
                palette: None,
                module: "typography".into(),
                model_override: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, LogoPipelineError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn emits_n_variants_deterministically() {
        let p = StubLogoPipeline::new();
        let a = p
            .generate_variants(LogoInput {
                prompt: "acme".into(),
                style: LogoStyle::Wordmark,
                count: 4,
                palette: None,
                module: "typography".into(),
                model_override: None,
            })
            .await
            .unwrap();
        let b = p
            .generate_variants(LogoInput {
                prompt: "acme".into(),
                style: LogoStyle::Wordmark,
                count: 4,
                palette: None,
                module: "typography".into(),
                model_override: None,
            })
            .await
            .unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 4);
        assert_eq!(a[0].model, "StubIdeogram");
    }

    #[tokio::test]
    async fn clamps_count_to_upper_bound() {
        let p = StubLogoPipeline::new();
        let v = p
            .generate_variants(LogoInput {
                prompt: "x".into(),
                style: LogoStyle::Mascot,
                count: 99,
                palette: None,
                module: "typography".into(),
                model_override: None,
            })
            .await
            .unwrap();
        assert_eq!(v.len(), 10);
    }
}
