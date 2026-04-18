//! Deterministic stub used by tests and as the default backend before a
//! Kling API key is configured. Mirrors `StubMeshPipeline`.

use async_trait::async_trait;

use super::types::{
    VideoImageInput, VideoPipeline, VideoPipelineError, VideoResult, VideoTextInput,
};

#[derive(Default)]
pub struct StubVideoPipeline;

impl StubVideoPipeline {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl VideoPipeline for StubVideoPipeline {
    async fn generate_from_text(
        &self,
        input: VideoTextInput,
    ) -> Result<VideoResult, VideoPipelineError> {
        if input.prompt.trim().is_empty() {
            return Err(VideoPipelineError::InvalidInput("prompt empty".into()));
        }
        Ok(VideoResult {
            video_url: format!("stub://video/text/{}.mp4", input.prompt.len()),
            local_path: None,
            model: "StubKlingText".into(),
            duration_s: input.duration_s,
        })
    }

    async fn generate_from_image(
        &self,
        input: VideoImageInput,
    ) -> Result<VideoResult, VideoPipelineError> {
        if input.image_url.trim().is_empty() {
            return Err(VideoPipelineError::InvalidInput(
                "image_url required".into(),
            ));
        }
        if input.image_url.starts_with("data:") {
            return Err(VideoPipelineError::InvalidInput(
                "data-URL unsupported".into(),
            ));
        }
        Ok(VideoResult {
            video_url: format!("stub://video/image/{}.mp4", input.image_url.len()),
            local_path: None,
            model: "StubKlingImage".into(),
            duration_s: input.duration_s,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn text_returns_deterministic_url() {
        let p = StubVideoPipeline::new();
        let a = p
            .generate_from_text(VideoTextInput {
                prompt: "a sunset".into(),
                duration_s: Some(5.0),
                module: None,
            })
            .await
            .unwrap();
        let b = p
            .generate_from_text(VideoTextInput {
                prompt: "a sunset".into(),
                duration_s: Some(5.0),
                module: None,
            })
            .await
            .unwrap();
        assert_eq!(a.video_url, b.video_url);
        assert_eq!(a.model, "StubKlingText");
        assert!(a.local_path.is_none());
        assert_eq!(a.duration_s, Some(5.0));
    }

    #[tokio::test]
    async fn text_rejects_empty_prompt() {
        let p = StubVideoPipeline::new();
        let err = p
            .generate_from_text(VideoTextInput {
                prompt: "   ".into(),
                duration_s: None,
                module: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, VideoPipelineError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn image_rejects_data_url() {
        let p = StubVideoPipeline::new();
        let err = p
            .generate_from_image(VideoImageInput {
                image_url: "data:image/png;base64,xyz".into(),
                prompt: None,
                duration_s: None,
                module: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, VideoPipelineError::InvalidInput(_)));
    }
}
