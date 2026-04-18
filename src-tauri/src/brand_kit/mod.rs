//! Brand-kit assembly: raster resize + color variants for logo exports.
//!
//! Given a source SVG string and raster PNG, produces a bundle of PNGs at
//! standard favicon/web/print sizes, plus grayscale and inverted variants,
//! plus a pass-through SVG, plus a style-guide HTML string.

pub mod commands;
pub mod export;
pub mod pipeline;
pub mod style_guide;
pub mod types;

pub use pipeline::StandardBrandKit;
pub use types::{BrandKitAsset, BrandKitBuilder, BrandKitError, BrandKitInput, BrandKitResult};
