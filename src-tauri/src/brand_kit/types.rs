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
///   to the style-guide generator.
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

// Manual `impl From` instead of thiserror's `#[from]` because the enum
// variants wrap `String` (for a stable, IPC-serializable wire shape) rather
// than the foreign error types directly. These impls give us the ergonomic
// `?` operator at call sites without changing the serde tag layout consumed
// by `BrandKitIpcError`.
impl From<std::io::Error> for BrandKitError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<image::ImageError> for BrandKitError {
    fn from(e: image::ImageError) -> Self {
        Self::Image(e.to_string())
    }
}

impl From<zip::result::ZipError> for BrandKitError {
    fn from(e: zip::result::ZipError) -> Self {
        Self::Io(e.to_string())
    }
}

/// Validate a [`BrandKitInput`] at the pipeline boundary.
///
/// Pairs with the escape helpers in [`super::style_guide`] to close the XSS
/// surface for Phase 8 consumers that may embed `style-guide.html` inside a
/// Tauri webview: here we reject any color that isn't a well-formed hex
/// literal, and there we escape every other string that reaches the HTML.
pub fn validate_input(input: &BrandKitInput) -> Result<(), BrandKitError> {
    validate_hex_color(&input.primary_color, "primary_color")?;
    validate_hex_color(&input.accent_color, "accent_color")?;
    Ok(())
}

fn validate_hex_color(value: &str, field: &str) -> Result<(), BrandKitError> {
    // Accept `#RGB`, `#RRGGBB`, `#RRGGBBAA` (case-insensitive). Reject
    // anything else — including named colors, `rgb(...)`, or raw hex
    // without the leading `#`.
    let rest = value.strip_prefix('#').ok_or_else(|| {
        BrandKitError::InvalidInput(format!(
            "{field} must be a hex color starting with '#', got {value:?}"
        ))
    })?;
    if !matches!(rest.len(), 3 | 6 | 8) {
        return Err(BrandKitError::InvalidInput(format!(
            "{field} must be 3, 6, or 8 hex digits after '#', got {value:?}"
        )));
    }
    if !rest.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(BrandKitError::InvalidInput(format!(
            "{field} contains non-hex characters, got {value:?}"
        )));
    }
    Ok(())
}

#[async_trait]
pub trait BrandKitBuilder: Send + Sync {
    async fn build(&self, input: BrandKitInput) -> Result<BrandKitResult, BrandKitError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input(primary: &str, accent: &str) -> BrandKitInput {
        BrandKitInput {
            logo_svg: "<svg/>".into(),
            source_png_path: PathBuf::from("x.png"),
            brand_name: "X".into(),
            primary_color: primary.into(),
            accent_color: accent.into(),
            font: "Inter".into(),
        }
    }

    #[test]
    fn validate_input_accepts_all_valid_hex_forms() {
        for (p, a) in &[
            ("#fff", "#000"),
            ("#FFF", "#000"),
            ("#E85D2D", "#0e0e11"),
            ("#FF0000AA", "#00FF0088"),
        ] {
            validate_input(&sample_input(p, a)).expect("should accept valid hex");
        }
    }

    #[test]
    fn validate_input_rejects_missing_hash() {
        let err = validate_input(&sample_input("e85d2d", "#000")).unwrap_err();
        assert!(matches!(err, BrandKitError::InvalidInput(_)));
    }

    #[test]
    fn validate_input_rejects_wrong_digit_count() {
        let err = validate_input(&sample_input("#ffff", "#000")).unwrap_err();
        assert!(matches!(err, BrandKitError::InvalidInput(_)));
    }

    #[test]
    fn validate_input_rejects_non_hex_chars() {
        let err = validate_input(&sample_input("#gggggg", "#000")).unwrap_err();
        assert!(matches!(err, BrandKitError::InvalidInput(_)));
    }

    #[test]
    fn validate_input_rejects_empty_accent() {
        let err = validate_input(&sample_input("#000", "")).unwrap_err();
        assert!(matches!(err, BrandKitError::InvalidInput(_)));
    }
}
