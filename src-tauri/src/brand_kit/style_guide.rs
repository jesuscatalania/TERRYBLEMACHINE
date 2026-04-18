//! Style-guide HTML generator.
//!
//! Emits a single-file HTML document with inline CSS (no external fonts,
//! scripts, or network dependencies) that renders the brand's logo, color
//! palette, typography specimen, and usage rules from the supplied
//! [`BrandKitInput`]. The output is self-contained so the frontend can
//! drop it straight into an iframe for preview, and T7's ZIP export can
//! ship it as `style-guide.html` without bundling extra assets.
//!
//! Intentionally no PDF rendering here — the backend stays dependency-free
//! on this path. If a PDF is needed, the frontend can render the HTML with
//! jsPDF (or print-to-PDF) against this same document.

use super::types::BrandKitInput;

pub fn build_style_guide(input: &BrandKitInput) -> String {
    let name = &input.brand_name;
    let primary = &input.primary_color;
    let accent = &input.accent_color;
    let font = &input.font;
    let svg = &input.logo_svg;

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{name} · Brand Guidelines</title>
<style>
  body {{ margin: 0; font-family: "{font}", sans-serif; color: #0E0E11; background: #F7F7F8; }}
  .container {{ max-width: 960px; margin: 0 auto; padding: 64px 32px; }}
  h1 {{ font-size: 4rem; margin: 0 0 0.25em; }}
  h2 {{ font-size: 1.125rem; text-transform: uppercase; letter-spacing: 0.08em; margin: 2em 0 1em; color: #6b6b70; }}
  .logo {{ width: 240px; height: 240px; padding: 32px; background: white; border: 1px solid #e5e5e5; }}
  .palette {{ display: flex; gap: 16px; }}
  .swatch {{ width: 140px; }}
  .swatch .chip {{ width: 100%; aspect-ratio: 1; border: 1px solid #e5e5e5; }}
  .swatch .meta {{ font-family: ui-monospace, "IBM Plex Mono", monospace; font-size: 0.75rem; padding-top: 8px; }}
  .specimen {{ font-size: 6rem; line-height: 1; margin: 0; }}
  .rules {{ font-size: 0.875rem; line-height: 1.6; color: #4b4b50; }}
  .rules li {{ margin-bottom: 0.5em; }}
</style>
</head>
<body>
<div class="container">
  <h1>{name}</h1>
  <p>Brand guidelines — v1.0</p>

  <h2>Logo</h2>
  <div class="logo">{svg}</div>
  <ul class="rules">
    <li>Minimum size: 24px height on screen, 12mm in print.</li>
    <li>Keep clear space equal to the height of the mark around all sides.</li>
    <li>Do not rotate, stretch, or recolor outside the provided variants.</li>
  </ul>

  <h2>Palette</h2>
  <div class="palette">
    <div class="swatch">
      <div class="chip" style="background: {primary};"></div>
      <div class="meta">Primary<br>{primary}</div>
    </div>
    <div class="swatch">
      <div class="chip" style="background: {accent};"></div>
      <div class="meta">Accent<br>{accent}</div>
    </div>
  </div>

  <h2>Typography</h2>
  <p class="specimen">{name}</p>
  <p class="rules">Typeface: {font}. Use for display and UI body text.</p>
</div>
</body>
</html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::super::types::BrandKitInput;
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn style_guide_embeds_brand_name_and_palette() {
        let input = BrandKitInput {
            logo_svg: "<svg></svg>".into(),
            source_png_path: PathBuf::from("x.png"),
            brand_name: "Acme".into(),
            primary_color: "#e85d2d".into(),
            accent_color: "#0E0E11".into(),
            font: "Inter".into(),
        };
        let html = build_style_guide(&input);
        assert!(html.contains("Acme"));
        assert!(html.contains("#e85d2d"));
        assert!(html.contains("#0E0E11"));
        assert!(html.contains("Inter"));
        assert!(html.contains("<svg>"));
    }
}
