//! Input/output types + trait for the video pipeline.
//!
//! Mirrors [`crate::mesh_pipeline::types`]: IPC-shaped `Deserialize` inputs,
//! a serializable result carrying both the remote MP4 URL *and* the local
//! cache path (so the frontend can prefer `convertFileSrc` over HTTPS when
//! the download succeeded), a `thiserror` error, and the async trait that
//! both `RouterVideoPipeline` and `StubVideoPipeline` implement.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─── Inputs ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct VideoTextInput {
    pub prompt: String,
    #[serde(default)]
    pub duration_s: Option<f32>,
    #[serde(default)]
    pub module: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VideoImageInput {
    pub image_url: String,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub duration_s: Option<f32>,
    #[serde(default)]
    pub module: Option<String>,
}

// ─── Result ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VideoResult {
    /// Remote MP4 URL returned by the provider (Kling / Runway / Higgsfield
    /// via Phase 2 polling).
    pub video_url: String,
    /// Local cache path for the downloaded MP4, if the download succeeded.
    /// Frontends should prefer this (via Tauri's `convertFileSrc`) and fall
    /// back to `video_url` when `None`.
    #[serde(default)]
    pub local_path: Option<PathBuf>,
    /// Concrete model the router dispatched to (debug/observability).
    pub model: String,
    /// Duration in seconds — echoed from the input since Kling doesn't
    /// always report the actual clip length back to the caller.
    #[serde(default)]
    pub duration_s: Option<f32>,
}

// ─── Error ─────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum VideoPipelineError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("router error: {0}")]
    Router(String),

    #[error("provider returned no video URL")]
    NoOutput,

    #[error("download failed: {0}")]
    Download(String),

    #[error("cache error: {0}")]
    Cache(String),
}

// ─── Trait ─────────────────────────────────────────────────────────────

#[async_trait]
pub trait VideoPipeline: Send + Sync {
    async fn generate_from_text(
        &self,
        input: VideoTextInput,
    ) -> Result<VideoResult, VideoPipelineError>;

    async fn generate_from_image(
        &self,
        input: VideoImageInput,
    ) -> Result<VideoResult, VideoPipelineError>;
}
