//! Types for logo generation via Ideogram v3.
//!
//! Mirrors the shape of [`crate::image_pipeline::types`] /
//! [`crate::mesh_pipeline::types`]: IPC-shaped `Deserialize` input, a
//! serializable variant result carrying both the remote URL and the local
//! cache path (so the frontend can prefer `convertFileSrc` over HTTPS when
//! the download succeeded), a `thiserror` error, and the async trait both
//! `RouterLogoPipeline` and `StubLogoPipeline` implement.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ai_router::Model;

// ─── Style ────────────────────────────────────────────────────────────

/// Logo design archetype. Feeds into the prompt via [`LogoStyle::brief`] so
/// the same user prompt produces visually distinct variants at different
/// style settings (a minimalist wordmark vs. an emblem vs. a mascot logo).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LogoStyle {
    #[default]
    Minimalist,
    Wordmark,
    Emblem,
    Mascot,
}

impl LogoStyle {
    /// Short prompt fragment describing the visual direction of this style.
    /// Appended to the user prompt by [`crate::logo_pipeline::pipeline`].
    pub fn brief(&self) -> &'static str {
        match self {
            Self::Minimalist => "minimalist, clean geometry, negative space, single color emphasis",
            Self::Wordmark => {
                "wordmark style, bold custom typography, letterforms as the central visual"
            }
            Self::Emblem => {
                "emblem or badge style, circular/shield frame, contained visual hierarchy"
            }
            Self::Mascot => "mascot character, friendly figure, expressive features",
        }
    }
}

// ─── Input ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct LogoInput {
    /// User-supplied brand prompt (e.g. "TERRYBLEMACHINE, a creative studio").
    pub prompt: String,
    /// Style archetype to bias the generation toward.
    #[serde(default)]
    pub style: LogoStyle,
    /// Number of variants to request. Clamped to `[1, 10]` by the pipeline.
    #[serde(default = "default_count")]
    pub count: u32,
    /// Optional palette hint appended to the prompt (e.g. "monochrome",
    /// "warm earth tones"). Empty / whitespace-only strings are ignored.
    #[serde(default)]
    pub palette: Option<String>,
    /// Module tag for taste-engine context matching (`"typography"`).
    #[serde(default = "default_module")]
    pub module: String,
    /// Optional model slug override from UI (ToolDropdown or `/tool`
    /// prefix). PascalCase variant name — matches `Model`'s default
    /// Serde repr (e.g. `"IdeogramV3"`). `None` means the router
    /// strategy picks the primary model. Every variant in the resulting
    /// spawn burst receives this override.
    #[serde(default)]
    pub model_override: Option<Model>,
}

fn default_count() -> u32 {
    5
}

fn default_module() -> String {
    "typography".to_string()
}

// ─── Result ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LogoVariant {
    /// Remote URL returned by Ideogram (or `file://` in tests).
    pub url: String,
    /// Local cache path for the downloaded PNG, if the download succeeded.
    /// Frontends should prefer this (via Tauri's `convertFileSrc`) and fall
    /// back to `url` when `None`.
    #[serde(default)]
    pub local_path: Option<PathBuf>,
    /// Seed used to salt this variant's request payload so each variant is
    /// a distinct cache key and renders as a distinct image.
    #[serde(default)]
    pub seed: Option<u32>,
    /// Concrete model the router dispatched to (debug/observability).
    pub model: String,
}

// ─── Error ─────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum LogoPipelineError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("router error: {0}")]
    Router(String),

    #[error("provider returned no image URL")]
    NoOutput,

    #[error("download failed: {0}")]
    Download(String),

    #[error("cache error: {0}")]
    Cache(String),
}

// ─── Trait ─────────────────────────────────────────────────────────────

#[async_trait]
pub trait LogoPipeline: Send + Sync {
    async fn generate_variants(
        &self,
        input: LogoInput,
    ) -> Result<Vec<LogoVariant>, LogoPipelineError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_input_accepts_model_override() {
        let json = r#"{"prompt":"acme","model_override":"IdeogramV3"}"#;
        let parsed: LogoInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model_override, Some(Model::IdeogramV3));
    }

    #[test]
    fn logo_input_defaults_model_override_to_none() {
        let json = r#"{"prompt":"acme"}"#;
        let parsed: LogoInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model_override, None);
    }
}
