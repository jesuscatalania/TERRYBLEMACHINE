//! Input/output types + trait for the Shotstack video-assembly pipeline.
//!
//! Mirrors [`crate::mesh_pipeline::types`]: IPC-shaped `Deserialize` inputs,
//! a serializable result carrying both the remote MP4 URL *and* the local
//! cache path (frontends should prefer the local path via `convertFileSrc`
//! and fall back to the remote URL when the download failed), a `thiserror`
//! error, and the async trait both the production `ShotstackAssembler` and
//! the deterministic `StubAssembler` implement.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─── Inputs ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssemblyClip {
    /// Source URL of the clip's video asset.
    pub src: String,
    /// Start time of the clip on the timeline, in seconds.
    pub start_s: f32,
    /// Length (duration) of the clip on the timeline, in seconds.
    pub length_s: f32,
    /// Optional in-transition (e.g. "fade", "slideLeft"). Passed to Shotstack
    /// unchanged — refer to Shotstack's transition catalogue for valid values.
    #[serde(default)]
    pub transition_in: Option<String>,
    /// Optional out-transition. Same passthrough semantics as `transition_in`.
    #[serde(default)]
    pub transition_out: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssemblyInput {
    /// Ordered list of clips to assemble into a single track. Empty → error.
    pub clips: Vec<AssemblyClip>,
    /// Optional soundtrack URL layered across the whole timeline.
    #[serde(default)]
    pub soundtrack: Option<String>,
    /// Container format (default "mp4").
    #[serde(default = "default_format")]
    pub format: String,
    /// Output resolution preset (default "hd").
    #[serde(default = "default_resolution")]
    pub resolution: String,
}

fn default_format() -> String {
    "mp4".into()
}
fn default_resolution() -> String {
    "hd".into()
}

// ─── Result ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AssemblyResult {
    /// Shotstack render id (echoes the POST response). Useful for observability
    /// + downstream status queries from the frontend.
    pub render_id: String,
    /// Remote MP4 URL returned by Shotstack once the render is `done`.
    pub video_url: String,
    /// Local cache path for the downloaded MP4, if the download succeeded.
    /// Frontends should prefer this (via Tauri's `convertFileSrc`) and fall
    /// back to `video_url` when `None`.
    #[serde(default)]
    pub local_path: Option<PathBuf>,
}

// ─── Error ─────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum AssemblyError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("provider error: {0}")]
    Provider(String),

    #[error("download failed: {0}")]
    Download(String),

    #[error("cache error: {0}")]
    Cache(String),
}

// ─── Trait ─────────────────────────────────────────────────────────────

#[async_trait]
pub trait VideoAssembler: Send + Sync {
    async fn assemble(&self, input: AssemblyInput) -> Result<AssemblyResult, AssemblyError>;
}
