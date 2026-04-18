//! Input/output types + trait for the depth-map pipeline.
//!
//! Mirrors the layout of [`crate::image_pipeline::types`]: a thin `Deserialize`
//! input struct shaped for Tauri IPC, a serializable result, a `thiserror`
//! error type, and the async trait that both `RouterDepthPipeline` and
//! `StubDepthPipeline` implement.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─── Input ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct DepthInput {
    /// URL (or `file://…`) pointing at the source image. Data-URLs are
    /// rejected at the pipeline boundary because Replicate's depth-anything
    /// endpoint cannot ingest them.
    pub image_url: String,
    /// Optional module tag (`"graphic3d"`) for future taste-engine routing;
    /// depth maps are currently module-agnostic.
    #[serde(default)]
    pub module: Option<String>,
}

// ─── Result ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepthResult {
    /// URL of the generated depth-map PNG (single-channel, brighter = closer).
    pub depth_url: String,
    /// The concrete model the router ended up calling (for debugging).
    pub model: String,
    #[serde(default)]
    pub cached: bool,
}

// ─── Error ─────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum DepthPipelineError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("router error: {0}")]
    Router(String),

    #[error("provider returned no depth URL")]
    NoOutput,
}

// ─── Trait ─────────────────────────────────────────────────────────────

#[async_trait]
pub trait DepthPipeline: Send + Sync {
    async fn generate(&self, input: DepthInput) -> Result<DepthResult, DepthPipelineError>;
}
