//! Style-guide HTML generator.
//!
//! T5 ships a placeholder that returns an empty HTML document so the
//! surrounding pipeline and IPC wiring can be tested end-to-end without
//! depending on the real generator. T6 overwrites this file with the
//! real implementation that renders brand colors, typography, and logo
//! examples against the `BrandKitInput` metadata.

use super::types::BrandKitInput;

pub fn build_style_guide(_input: &BrandKitInput) -> String {
    "<html></html>".to_string()
}
