//! Input/output types + trait for the mesh (3D) pipeline.
//!
//! Mirrors [`crate::depth_pipeline::types`]: IPC-shaped `Deserialize` inputs,
//! a serializable result carrying both the remote GLB URL *and* the local
//! cache path (so the frontend can prefer `convertFileSrc` over HTTPS when
//! the download succeeded), a `thiserror` error, and the async trait that
//! both `RouterMeshPipeline` and `StubMeshPipeline` implement.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ai_router::Model;

// ─── Inputs ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct MeshTextInput {
    pub prompt: String,
    #[serde(default)]
    pub module: Option<String>,
    /// Optional model slug override from UI (ToolDropdown or `/tool`
    /// prefix). PascalCase variant name — matches `Model`'s default
    /// Serde repr (e.g. `"MeshyText3D"`). `None` means the router
    /// strategy picks the primary model.
    #[serde(default)]
    pub model_override: Option<Model>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MeshImageInput {
    pub image_url: String,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub module: Option<String>,
    /// When `true`, routes through `Complexity::Simple` so the AI router
    /// picks `ReplicateTripoSR` (fast + cheap) as the primary and Meshy as
    /// the fallback. Defaults to `false` → Meshy primary, TripoSR fallback.
    #[serde(default)]
    pub quick_preview: bool,
}

// ─── Result ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MeshResult {
    /// Remote GLB URL returned by the provider.
    pub glb_url: String,
    /// Local cache path for the downloaded GLB, if the download succeeded.
    /// Frontends should prefer this (via Tauri's `convertFileSrc`) and fall
    /// back to `glb_url` when `None`.
    #[serde(default)]
    pub local_path: Option<PathBuf>,
    /// Concrete model the router dispatched to (debug/observability).
    pub model: String,
}

// ─── Error ─────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum MeshPipelineError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("router error: {0}")]
    Router(String),

    #[error("provider returned no GLB URL")]
    NoOutput,

    #[error("download failed: {0}")]
    Download(String),

    #[error("cache error: {0}")]
    Cache(String),
}

// ─── Trait ─────────────────────────────────────────────────────────────

#[async_trait]
pub trait MeshPipeline: Send + Sync {
    async fn generate_from_text(
        &self,
        input: MeshTextInput,
    ) -> Result<MeshResult, MeshPipelineError>;

    async fn generate_from_image(
        &self,
        input: MeshImageInput,
    ) -> Result<MeshResult, MeshPipelineError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_text_input_accepts_model_override() {
        let json = r#"{"prompt":"a cube","model_override":"MeshyText3D"}"#;
        let parsed: MeshTextInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model_override, Some(Model::MeshyText3D));
    }

    #[test]
    fn mesh_text_input_defaults_model_override_to_none() {
        let json = r#"{"prompt":"a cube"}"#;
        let parsed: MeshTextInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model_override, None);
    }
}
