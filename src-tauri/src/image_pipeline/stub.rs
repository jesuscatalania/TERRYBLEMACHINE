//! Deterministic stub used by tests and as the default backend before Phase
//! 4.2 wires a real AI router with provider credentials.

use std::sync::Mutex;

use async_trait::async_trait;

use super::types::{
    GenerateVariantsInput, Image2ImageInput, ImagePipeline, ImagePipelineError, ImageResult,
    Text2ImageInput, UpscaleInput,
};

#[derive(Default)]
pub struct StubImagePipeline {
    force_error: Mutex<Option<String>>,
    calls: Mutex<Vec<String>>,
}

impl StubImagePipeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn force_error(&self, message: impl Into<String>) {
        *self.force_error.lock().expect("poisoned") = Some(message.into());
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("poisoned").clone()
    }

    fn record(&self, op: &str) {
        self.calls.lock().expect("poisoned").push(op.to_string());
    }

    fn check_forced(&self) -> Result<(), ImagePipelineError> {
        if let Some(msg) = self.force_error.lock().expect("poisoned").clone() {
            return Err(ImagePipelineError::Router(msg));
        }
        Ok(())
    }
}

fn fake_result(prompt: &str, seed: u64) -> ImageResult {
    ImageResult {
        url: format!("stub://image/{}/{seed}", slug(prompt)),
        width: Some(1024),
        height: Some(1024),
        seed: Some(seed),
        model: "stub".to_string(),
        cached: false,
    }
}

fn slug(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    cleaned
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[async_trait]
impl ImagePipeline for StubImagePipeline {
    async fn text_to_image(
        &self,
        input: Text2ImageInput,
    ) -> Result<ImageResult, ImagePipelineError> {
        self.check_forced()?;
        self.record("text_to_image");
        if input.prompt.trim().is_empty() {
            return Err(ImagePipelineError::InvalidInput("empty prompt".into()));
        }
        Ok(fake_result(&input.prompt, 1))
    }

    async fn image_to_image(
        &self,
        input: Image2ImageInput,
    ) -> Result<ImageResult, ImagePipelineError> {
        self.check_forced()?;
        self.record("image_to_image");
        if input.prompt.trim().is_empty() || input.image_url.trim().is_empty() {
            return Err(ImagePipelineError::InvalidInput(
                "prompt and image_url required".into(),
            ));
        }
        Ok(fake_result(&input.prompt, 2))
    }

    async fn upscale(&self, input: UpscaleInput) -> Result<ImageResult, ImagePipelineError> {
        self.check_forced()?;
        self.record("upscale");
        if input.image_url.trim().is_empty() {
            return Err(ImagePipelineError::InvalidInput(
                "image_url required".into(),
            ));
        }
        let mut r = fake_result(&input.image_url, 3);
        r.width = Some(1024 * input.scale);
        r.height = Some(1024 * input.scale);
        Ok(r)
    }

    async fn variants(
        &self,
        input: GenerateVariantsInput,
    ) -> Result<Vec<ImageResult>, ImagePipelineError> {
        self.check_forced()?;
        self.record("variants");
        if input.prompt.trim().is_empty() {
            return Err(ImagePipelineError::InvalidInput("empty prompt".into()));
        }
        let n = input.count.clamp(1, 8);
        Ok((0..n)
            .map(|i| fake_result(&input.prompt, 100 + i as u64))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_router::Complexity;

    fn t2i(prompt: &str) -> Text2ImageInput {
        Text2ImageInput {
            prompt: prompt.into(),
            complexity: Complexity::Medium,
            module: "graphic2d".into(),
        }
    }

    #[tokio::test]
    async fn text_to_image_returns_stub_url_for_valid_prompt() {
        let p = StubImagePipeline::new();
        let r = p.text_to_image(t2i("a red rose")).await.unwrap();
        assert!(r.url.contains("a-red-rose"));
        assert_eq!(r.width, Some(1024));
    }

    #[tokio::test]
    async fn empty_prompt_is_invalid_input() {
        let p = StubImagePipeline::new();
        let err = p.text_to_image(t2i("   ")).await.unwrap_err();
        assert!(matches!(err, ImagePipelineError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn variants_return_requested_count_clamped_to_eight() {
        let p = StubImagePipeline::new();
        let results = p
            .variants(GenerateVariantsInput {
                prompt: "flowers".into(),
                count: 4,
                complexity: Complexity::Medium,
                module: "graphic2d".into(),
            })
            .await
            .unwrap();
        assert_eq!(results.len(), 4);

        let many = p
            .variants(GenerateVariantsInput {
                prompt: "x".into(),
                count: 99,
                complexity: Complexity::Medium,
                module: "graphic2d".into(),
            })
            .await
            .unwrap();
        assert_eq!(many.len(), 8);
    }

    #[tokio::test]
    async fn upscale_doubles_dimensions_for_scale_2() {
        let p = StubImagePipeline::new();
        let r = p
            .upscale(UpscaleInput {
                image_url: "stub://a".into(),
                scale: 2,
            })
            .await
            .unwrap();
        assert_eq!(r.width, Some(2048));
    }

    #[tokio::test]
    async fn force_error_propagates_as_router_error() {
        let p = StubImagePipeline::new();
        p.force_error("boom");
        let err = p.text_to_image(t2i("x")).await.unwrap_err();
        assert!(matches!(err, ImagePipelineError::Router(_)));
    }

    #[tokio::test]
    async fn calls_are_recorded_in_order() {
        let p = StubImagePipeline::new();
        p.text_to_image(t2i("a")).await.unwrap();
        p.upscale(UpscaleInput {
            image_url: "stub://x".into(),
            scale: 2,
        })
        .await
        .unwrap();
        assert_eq!(p.calls(), vec!["text_to_image", "upscale"]);
    }
}
