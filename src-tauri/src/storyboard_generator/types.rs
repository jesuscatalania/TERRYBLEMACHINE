//! Type definitions for the storyboard generator.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ai_router::Model;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StoryboardTemplate {
    #[default]
    Commercial,
    Explainer,
    SocialMedia,
    MusicVideo,
    Custom,
}

impl StoryboardTemplate {
    pub fn brief(&self) -> &'static str {
        match self {
            Self::Commercial => "a 20-40 second product commercial with clear call-to-action",
            Self::Explainer => "a 45-90 second explainer: problem, solution, product, outcome",
            Self::SocialMedia => "a 15-30 second social-media spot, punchy hook in first 3 seconds",
            Self::MusicVideo => "a music video cut to beat; visual motif > narrative",
            Self::Custom => "",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StoryboardInput {
    pub prompt: String,
    #[serde(default)]
    pub template: StoryboardTemplate,
    #[serde(default = "default_module")]
    pub module: String,
    /// Optional model slug override from UI (ToolDropdown or `/tool`
    /// prefix). PascalCase variant name — matches `Model`'s default
    /// Serde repr (e.g. `"FalKlingV2Master"`). `None` means the router
    /// strategy picks the primary model for the storyboard's
    /// text-generation dispatch.
    #[serde(default)]
    pub model_override: Option<Model>,
}

fn default_module() -> String {
    "video".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shot {
    pub index: u32,
    pub description: String,
    pub style: String,
    pub duration_s: f32,
    pub camera: String,
    pub transition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storyboard {
    pub summary: String,
    pub template: String,
    pub shots: Vec<Shot>,
}

#[derive(Debug, Error)]
pub enum StoryboardError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("router error: {0}")]
    Router(String),
    #[error("failed to parse storyboard JSON: {0}")]
    Parse(String),
}

#[async_trait]
pub trait StoryboardGenerator: Send + Sync {
    async fn generate(&self, input: StoryboardInput) -> Result<Storyboard, StoryboardError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storyboard_input_accepts_model_override() {
        let json = r#"{"prompt":"sunrise","model_override":"FalKlingV2Master"}"#;
        let parsed: StoryboardInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model_override, Some(Model::FalKlingV2Master));
    }

    #[test]
    fn storyboard_input_defaults_model_override_to_none() {
        let json = r#"{"prompt":"sunrise"}"#;
        let parsed: StoryboardInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model_override, None);
    }
}
