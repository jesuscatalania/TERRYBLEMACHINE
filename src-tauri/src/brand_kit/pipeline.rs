//! Production [`BrandKitBuilder`] — resizes the source PNG into every
//! favicon/web/print size, derives grayscale + inverted variants, and
//! passes the source SVG through unchanged. All raster ops go through
//! the `image` crate; the SVG stays as a byte copy because a full vector
//! re-render would lose any manual edits the user made in T4's SvgEditor.
//!
//! The pipeline is in-memory only: we read the source PNG into a
//! [`image::DynamicImage`] once and re-encode a PNG per size into a
//! `Vec<u8>`. T7 will be responsible for writing these to a ZIP on disk.
//!
//! For a 4096×4096 source PNG the decode + 8× Lanczos3 resize + 2 variant
//! passes + 10 PNG encodes is seconds of CPU work, so the hot path runs
//! inside [`tokio::task::spawn_blocking`] to keep the runtime responsive.

use async_trait::async_trait;

use super::types::{BrandKitAsset, BrandKitBuilder, BrandKitError, BrandKitInput, BrandKitResult};

pub struct StandardBrandKit;

impl StandardBrandKit {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StandardBrandKit {
    fn default() -> Self {
        Self::new()
    }
}

/// Target raster sizes. Labels double as output filenames (suffixed `.png`)
/// so the consumer (T7 ZIP) doesn't need a separate naming convention.
const SIZES: &[(u32, &str)] = &[
    (16, "favicon-16"),
    (32, "favicon-32"),
    (64, "favicon-64"),
    (128, "logo-128"),
    (256, "logo-256"),
    (512, "logo-512"),
    (1024, "print-1024"),
    (2048, "print-2048"),
];

#[async_trait]
impl BrandKitBuilder for StandardBrandKit {
    async fn build(&self, input: BrandKitInput) -> Result<BrandKitResult, BrandKitError> {
        // Cheap validation stays on the Tokio thread so rejection paths
        // don't pay for a spawn_blocking hop.
        if input.logo_svg.trim().is_empty() {
            return Err(BrandKitError::InvalidInput("logo_svg empty".into()));
        }
        if !input.source_png_path.exists() {
            return Err(BrandKitError::InvalidInput(format!(
                "source_png missing: {}",
                input.source_png_path.display()
            )));
        }

        // Style-guide generation is trivial (string concat) for now — pulled
        // out to avoid forcing extra clones into the blocking closure.
        let style_guide_html = super::style_guide::build_style_guide(&input);

        let source_png_path = input.source_png_path;
        let logo_svg_bytes = input.logo_svg.into_bytes();

        let assets =
            tokio::task::spawn_blocking(move || -> Result<Vec<BrandKitAsset>, BrandKitError> {
                let source_bytes = std::fs::read(&source_png_path)
                    .map_err(|e| BrandKitError::Io(e.to_string()))?;
                let img = image::load_from_memory(&source_bytes)
                    .map_err(|e| BrandKitError::Image(e.to_string()))?;

                let mut assets: Vec<BrandKitAsset> = Vec::new();

                // Pass-through SVG — preserves any edits made upstream.
                assets.push(BrandKitAsset {
                    filename: "logo.svg".into(),
                    bytes: logo_svg_bytes,
                });

                // Raster sizes — original palette, Lanczos3 resample for
                // quality at downscales (favicons) without losing sharpness
                // on upscales (print).
                for &(size, label) in SIZES {
                    let resized = img.resize(size, size, image::imageops::FilterType::Lanczos3);
                    let mut buf = Vec::new();
                    let mut cursor = std::io::Cursor::new(&mut buf);
                    resized
                        .write_to(&mut cursor, image::ImageFormat::Png)
                        .map_err(|e| BrandKitError::Image(e.to_string()))?;
                    assets.push(BrandKitAsset {
                        filename: format!("{label}.png"),
                        bytes: buf,
                    });
                }

                // B&W variant. We compute Rec.601 luma per pixel on an RGBA8
                // buffer so the alpha channel survives — `DynamicImage::grayscale`
                // would drop the alpha for logos on transparent backgrounds.
                let mut bw = img.to_rgba8();
                for pixel in bw.pixels_mut() {
                    // Rec.601 luma: 0.299R + 0.587G + 0.114B
                    let r = pixel.0[0] as u32;
                    let g = pixel.0[1] as u32;
                    let b = pixel.0[2] as u32;
                    let luma = ((299 * r + 587 * g + 114 * b + 500) / 1000) as u8;
                    pixel.0[0] = luma;
                    pixel.0[1] = luma;
                    pixel.0[2] = luma;
                    // pixel.0[3] (alpha) untouched
                }
                let mut bw_buf = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut bw_buf);
                image::DynamicImage::ImageRgba8(bw)
                    .write_to(&mut cursor, image::ImageFormat::Png)
                    .map_err(|e| BrandKitError::Image(e.to_string()))?;
                assets.push(BrandKitAsset {
                    filename: "logo-bw.png".into(),
                    bytes: bw_buf,
                });

                // Inverted variant. Alpha is preserved; only RGB channels flip.
                let mut inv = img.to_rgba8();
                for pixel in inv.pixels_mut() {
                    pixel.0[0] = 255 - pixel.0[0];
                    pixel.0[1] = 255 - pixel.0[1];
                    pixel.0[2] = 255 - pixel.0[2];
                }
                let mut inv_buf = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut inv_buf);
                image::DynamicImage::ImageRgba8(inv)
                    .write_to(&mut cursor, image::ImageFormat::Png)
                    .map_err(|e| BrandKitError::Image(e.to_string()))?;
                assets.push(BrandKitAsset {
                    filename: "logo-inverted.png".into(),
                    bytes: inv_buf,
                });

                Ok(assets)
            })
            .await
            .map_err(|e| BrandKitError::Image(format!("join error: {e}")))??;

        Ok(BrandKitResult {
            assets,
            style_guide_html,
        })
    }
}
