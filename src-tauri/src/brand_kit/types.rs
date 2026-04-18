//! Types for the brand-kit assembly pipeline.
//!
//! Mirrors the shape of sibling pipeline modules (`vectorizer::types`,
//! `logo_pipeline::types`): IPC-shaped `Deserialize` input carrying the
//! source SVG/raster plus brand metadata, a serializable `BrandKitResult`
//! that owns every generated asset in-memory (T7 will write them to a ZIP;
//! keeping them as `Vec<u8>` at this layer avoids juggling temp dirs at
//! the IPC boundary), a `thiserror` error, and the async trait the
//! [`StandardBrandKit`](super::pipeline::StandardBrandKit) implements.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// User-supplied brand-kit inputs.
///
/// - `logo_svg` is the vector markup (usually from T3's vectorizer output)
///   and gets passed through unchanged as `logo.svg`.
/// - `source_png_path` is a raster render of the same logo; the pipeline
///   resizes it into every favicon/web/print size and derives the B&W and
///   inverted variants from it.
/// - `brand_name`, `primary_color`, `accent_color`, `font` are forwarded
///   to the style-guide generator (placeholder in T5, real in T6).
#[derive(Debug, Clone, Deserialize)]
pub struct BrandKitInput {
    pub logo_svg: String,
    pub source_png_path: PathBuf,
    pub brand_name: String,
    pub primary_color: String,
    pub accent_color: String,
    pub font: String,
}

/// A single generated asset. `filename` is the name the consumer (T7 ZIP)
/// should use; `bytes` is the raw file content.
#[derive(Debug, Clone, Serialize)]
pub struct BrandKitAsset {
    pub filename: String,
    pub bytes: Vec<u8>,
}

/// Full brand-kit bundle: all assets plus an HTML style-guide string that
/// T6 will fill in. Keeping `style_guide_html` as a string (not an asset)
/// makes it trivial for the frontend preview to render it inline before
/// the bundle gets zipped.
#[derive(Debug, Clone, Serialize)]
pub struct BrandKitResult {
    pub assets: Vec<BrandKitAsset>,
    pub style_guide_html: String,
}

#[derive(Debug, Error)]
pub enum BrandKitError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("image error: {0}")]
    Image(String),
    #[error("io error: {0}")]
    Io(String),
}

#[async_trait]
pub trait BrandKitBuilder: Send + Sync {
    async fn build(&self, input: BrandKitInput) -> Result<BrandKitResult, BrandKitError>;
}
