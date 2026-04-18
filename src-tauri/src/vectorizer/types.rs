//! Types for raster→SVG vectorization via VTracer.
//!
//! Mirrors the shape of [`crate::logo_pipeline::types`] /
//! [`crate::mesh_pipeline::types`]: IPC-shaped `Deserialize` input, a
//! serializable `VectorizeResult` carrying the raw SVG markup and the
//! parsed canvas dimensions, a `thiserror` error, and the async trait
//! both [`VtracerPipeline`](super::pipeline::VtracerPipeline) and
//! [`StubVectorizer`](super::stub::StubVectorizer) implement.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─── Input ─────────────────────────────────────────────────────────────

/// Which VTracer tracing mode to use. Serialized as kebab-case (`"color"` /
/// `"bw"`) so the frontend IPC payload stays stable and typos cause a
/// Deserialize error rather than silently falling through to `Color`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorMode {
    #[default]
    Color,
    Bw,
}

/// User-supplied vectorization parameters.
///
/// The three tunables expose VTracer's most-used knobs directly:
/// - `color_mode` — [`ColorMode::Color`] traces the full palette (logos,
///   illustrations), [`ColorMode::Bw`] flattens to binary (line art, icons).
/// - `filter_speckle` — drops clusters smaller than `n×n` px. Higher = cleaner,
///   lower = preserves fine detail. VTracer default is `4`, max `128`.
/// - `corner_threshold` — degrees below which a vertex is simplified to a
///   straight segment. VTracer default is `60`, max `180`.
#[derive(Debug, Clone, Deserialize)]
pub struct VectorizeInput {
    pub image_path: PathBuf,
    #[serde(default)]
    pub color_mode: ColorMode,
    #[serde(default = "default_filter_speckle")]
    pub filter_speckle: u32,
    #[serde(default = "default_corner_threshold")]
    pub corner_threshold: u32,
}

fn default_filter_speckle() -> u32 {
    4
}
fn default_corner_threshold() -> u32 {
    60
}

/// Shared bounds check run by every [`Vectorizer`] impl before invoking a
/// provider. Keeps validation at the pipeline layer so both the real
/// [`VtracerPipeline`](super::pipeline::VtracerPipeline) and the test
/// [`StubVectorizer`](super::stub::StubVectorizer) reject obviously-bad
/// input the same way. VTracer itself accepts `corner_threshold` 0–180
/// (degrees) and `filter_speckle` 0–128 (cluster area).
pub fn validate_input(input: &VectorizeInput) -> Result<(), VectorizeError> {
    if input.corner_threshold > 180 {
        return Err(VectorizeError::InvalidInput(format!(
            "corner_threshold must be 0-180, got {}",
            input.corner_threshold
        )));
    }
    if input.filter_speckle > 128 {
        return Err(VectorizeError::InvalidInput(format!(
            "filter_speckle must be 0-128, got {}",
            input.filter_speckle
        )));
    }
    Ok(())
}

// ─── Result ────────────────────────────────────────────────────────────

/// Vectorized SVG plus parsed canvas dimensions. The frontend uses `width`
/// and `height` to size the SvgEditor viewport; `svg` is inlined directly
/// into the DOM so there's no extra round-trip to disk.
#[derive(Debug, Clone, Serialize)]
pub struct VectorizeResult {
    pub svg: String,
    pub width: u32,
    pub height: u32,
}

// ─── Error ─────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum VectorizeError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("vtracer error: {0}")]
    Vtracer(String),
    #[error("io error: {0}")]
    Io(String),
}

// ─── Trait ─────────────────────────────────────────────────────────────

#[async_trait]
pub trait Vectorizer: Send + Sync {
    async fn vectorize(&self, input: VectorizeInput) -> Result<VectorizeResult, VectorizeError>;
}
