use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ai_router::Model;
use crate::website_analyzer::AnalysisResult;

use super::templates::Template;

/// Everything needed to produce one generated project.
#[derive(Debug, Clone, Deserialize)]
pub struct GenerationInput {
    /// The user's free-text brief.
    pub prompt: String,
    /// Template scaffold to follow.
    #[serde(default)]
    pub template: Template,
    /// Optional reference URL analysis (from `website_analyzer`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<AnalysisResult>,
    /// Optional image asset path (e.g. mood board).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_path: Option<PathBuf>,
    /// Module identifier — used to pick matching context rules from
    /// `meingeschmack/`. Defaults to `"website"`.
    #[serde(default = "default_module")]
    pub module: String,
    /// Optional model slug override from UI (ToolDropdown or `/tool`
    /// prefix). PascalCase variant name — matches `Model`'s default
    /// Serde repr (e.g. `"ClaudeSonnet"`). `None` means the router
    /// strategy picks the primary model.
    #[serde(default)]
    pub model_override: Option<Model>,
}

fn default_module() -> String {
    "website".to_string()
}

/// One file in the generated project, path relative to the project root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedFile {
    pub path: PathBuf,
    pub content: String,
}

/// Full output returned by a [`CodeGenerator`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedProject {
    /// A short description of what was generated (for UI status messages).
    pub summary: String,
    /// Every file in the project (index.html, components, styles, …).
    pub files: Vec<GeneratedFile>,
    /// The prompt actually sent upstream — useful for diagnostics + replay.
    pub prompt: String,
}

impl GeneratedProject {
    pub fn file(&self, relative: impl AsRef<std::path::Path>) -> Option<&GeneratedFile> {
        let rel = relative.as_ref();
        self.files.iter().find(|f| f.path == rel)
    }
}

#[derive(Debug, Error)]
pub enum CodeGenError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("provider error: {0}")]
    Provider(String),

    #[error("malformed response: {0}")]
    ParseResponse(String),
}

#[async_trait]
pub trait CodeGenerator: Send + Sync {
    async fn generate(&self, input: GenerationInput) -> Result<GeneratedProject, CodeGenError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_input_accepts_model_override() {
        let json = r#"{"prompt":"a landing page","model_override":"ClaudeSonnet"}"#;
        let parsed: GenerationInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model_override, Some(Model::ClaudeSonnet));
    }

    #[test]
    fn generation_input_defaults_model_override_to_none() {
        let json = r#"{"prompt":"a landing page"}"#;
        let parsed: GenerationInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model_override, None);
    }
}
