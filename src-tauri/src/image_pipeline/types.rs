use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ai_router::{Complexity, Model};

// ─── Inputs ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Text2ImageInput {
    pub prompt: String,
    #[serde(default = "default_complexity")]
    pub complexity: Complexity,
    /// Module tag for taste-engine context matching (`"graphic2d"`).
    #[serde(default = "default_module")]
    pub module: String,
    /// Optional model slug override from UI (ToolDropdown or `/tool`
    /// prefix). PascalCase variant name — matches `Model`'s default
    /// Serde repr (e.g. `"FalFluxPro"`). `None` means router strategy
    /// picks the primary model.
    #[serde(default)]
    pub model_override: Option<Model>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Image2ImageInput {
    pub prompt: String,
    /// URL (or `file://…`) pointing at the source image.
    pub image_url: String,
    #[serde(default = "default_complexity")]
    pub complexity: Complexity,
    #[serde(default = "default_module")]
    pub module: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpscaleInput {
    pub image_url: String,
    /// 2 or 4 are the Real-ESRGAN conventions. Defaults to 2.
    #[serde(default = "default_scale")]
    pub scale: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateVariantsInput {
    pub prompt: String,
    /// Number of parallel variants to request. Clamped to [1, 8].
    #[serde(default = "default_variant_count")]
    pub count: u32,
    #[serde(default = "default_complexity")]
    pub complexity: Complexity,
    #[serde(default = "default_module")]
    pub module: String,
    /// Optional model slug override (same semantics as
    /// [`Text2ImageInput::model_override`]). Every variant in the
    /// resulting spawn burst receives this override.
    #[serde(default)]
    pub model_override: Option<Model>,
}

/// Inpainting: replace the masked region of `source_url` with content
/// generated from `prompt`. The mask is a same-size image where opaque
/// (white) pixels mark the region to regenerate.
#[derive(Debug, Clone, Deserialize)]
pub struct InpaintInput {
    pub prompt: String,
    /// URL (or `file://…`) pointing at the source image. fal.ai flux-fill
    /// requires a publicly-hosted URL; data-URLs will fail at the provider.
    pub source_url: String,
    /// URL pointing at a same-size mask image (white = repaint region).
    pub mask_url: String,
    #[serde(default = "default_complexity")]
    pub complexity: Complexity,
    #[serde(default = "default_module")]
    pub module: String,
}

fn default_complexity() -> Complexity {
    Complexity::Medium
}
fn default_module() -> String {
    "graphic2d".to_string()
}
fn default_scale() -> u32 {
    2
}
fn default_variant_count() -> u32 {
    4
}

// ─── Result ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageResult {
    /// URL of the generated image. For fal.ai this is a cdn.fal.ai URL.
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    #[serde(default)]
    pub seed: Option<u64>,
    /// The concrete model the router ended up calling (for debugging).
    pub model: String,
    #[serde(default)]
    pub cached: bool,
}

// ─── Error ─────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ImagePipelineError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("router error: {0}")]
    Router(String),

    #[error("no image URL in response")]
    EmptyResponse,

    #[error("all {0} variants failed")]
    AllVariantsFailed(u32),
}

// ─── Trait ─────────────────────────────────────────────────────────────

#[async_trait]
pub trait ImagePipeline: Send + Sync {
    async fn text_to_image(
        &self,
        input: Text2ImageInput,
    ) -> Result<ImageResult, ImagePipelineError>;
    async fn image_to_image(
        &self,
        input: Image2ImageInput,
    ) -> Result<ImageResult, ImagePipelineError>;
    async fn upscale(&self, input: UpscaleInput) -> Result<ImageResult, ImagePipelineError>;
    async fn variants(
        &self,
        input: GenerateVariantsInput,
    ) -> Result<Vec<ImageResult>, ImagePipelineError>;
    async fn inpaint(&self, input: InpaintInput) -> Result<ImageResult, ImagePipelineError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_variants_input_accepts_model_override() {
        let json = r#"{"prompt":"a cat","model_override":"FalSdxl"}"#;
        let parsed: GenerateVariantsInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model_override, Some(Model::FalSdxl));
    }

    #[test]
    fn generate_variants_input_defaults_model_override_to_none() {
        let json = r#"{"prompt":"a cat"}"#;
        let parsed: GenerateVariantsInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model_override, None);
    }

    #[test]
    fn text2image_input_accepts_model_override() {
        let json = r#"{"prompt":"a cat","model_override":"FalFluxPro"}"#;
        let parsed: Text2ImageInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model_override, Some(Model::FalFluxPro));
    }
}
