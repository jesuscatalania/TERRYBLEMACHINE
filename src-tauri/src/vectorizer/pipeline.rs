//! Production [`Vectorizer`] backed by the `vtracer` crate.
//!
//! VTracer's `convert_image_to_svg` is synchronous and disk-based: it reads
//! the raster from a file and writes the resulting SVG back to disk. To keep
//! the Tokio runtime free we run the whole conversion inside
//! [`tokio::task::spawn_blocking`], using a [`NamedTempFile`] for the
//! intermediate SVG so we don't leak artifacts between calls.
//!
//! After VTracer writes the file we read it back into a `String` (so the IPC
//! layer can inline the markup into the frontend without a second file hop)
//! and parse out `width="…" height="…"` so the SvgEditor can size its
//! viewport. If either attribute is missing we fall back to a 1024×1024
//! canvas rather than erroring — VTracer always emits both in practice but
//! the parser is intentionally forgiving.

use std::fs;

use async_trait::async_trait;
use tempfile::NamedTempFile;

use super::types::{VectorizeError, VectorizeInput, VectorizeResult, Vectorizer};

pub struct VtracerPipeline;

impl VtracerPipeline {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VtracerPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Vectorizer for VtracerPipeline {
    async fn vectorize(&self, input: VectorizeInput) -> Result<VectorizeResult, VectorizeError> {
        if !input.image_path.exists() {
            return Err(VectorizeError::InvalidInput(format!(
                "image not found: {}",
                input.image_path.display()
            )));
        }

        let input_path = input.image_path.clone();
        let color_mode = input.color_mode.clone();
        let filter_speckle = input.filter_speckle;
        let corner_threshold = input.corner_threshold;

        // vtracer::convert_image_to_svg reads/writes synchronously — we run
        // the whole block off the Tokio runtime via spawn_blocking.
        let (svg, width, height) =
            tokio::task::spawn_blocking(move || -> Result<(String, u32, u32), VectorizeError> {
                let tmp = NamedTempFile::new().map_err(|e| VectorizeError::Io(e.to_string()))?;
                let out_path = tmp.path().to_owned();

                let config = vtracer::Config {
                    color_mode: if color_mode == "bw" {
                        vtracer::ColorMode::Binary
                    } else {
                        vtracer::ColorMode::Color
                    },
                    filter_speckle: filter_speckle as usize,
                    corner_threshold: corner_threshold as i32,
                    ..vtracer::Config::default()
                };

                vtracer::convert_image_to_svg(input_path.as_path(), out_path.as_path(), config)
                    .map_err(VectorizeError::Vtracer)?;

                let svg =
                    fs::read_to_string(&out_path).map_err(|e| VectorizeError::Io(e.to_string()))?;
                let (w, h) = parse_svg_dimensions(&svg).unwrap_or((1024, 1024));
                Ok((svg, w, h))
            })
            .await
            .map_err(|e| VectorizeError::Vtracer(format!("join error: {e}")))??;

        Ok(VectorizeResult { svg, width, height })
    }
}

/// Extract `width` and `height` attribute values from the opening `<svg>`
/// tag. VTracer emits integer pixel dimensions (no `px` suffix, no
/// floating-point), so a literal `attr="N"` scan is sufficient — we don't
/// need a full XML parser here. Returns `None` when either attribute is
/// missing or not a plain `u32`.
fn parse_svg_dimensions(svg: &str) -> Option<(u32, u32)> {
    let w = find_attr(svg, "width")?;
    let h = find_attr(svg, "height")?;
    Some((w, h))
}

fn find_attr(s: &str, attr: &str) -> Option<u32> {
    let needle = format!("{attr}=\"");
    let start = s.find(&needle)? + needle.len();
    let end = s[start..].find('"')?;
    s[start..start + end].parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_svg_dimensions_extracts_both() {
        let svg = r#"<svg width="100" height="200" xmlns="http://www.w3.org/2000/svg"></svg>"#;
        assert_eq!(parse_svg_dimensions(svg), Some((100, 200)));
    }

    #[test]
    fn parse_svg_dimensions_handles_missing() {
        assert_eq!(parse_svg_dimensions("<svg></svg>"), None);
    }

    #[test]
    fn parse_svg_dimensions_handles_non_numeric() {
        let svg = r#"<svg width="auto" height="200"></svg>"#;
        assert_eq!(parse_svg_dimensions(svg), None);
    }
}
