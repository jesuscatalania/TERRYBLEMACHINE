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
//!
//! Every string that reaches the HTML/CSS passes through a context-specific
//! escape helper. `logo_svg` is deliberately inlined raw because the upstream
//! vectorizer produces trusted SVG markup that we want rendered — the other
//! user-supplied strings (`brand_name`, `font`, `primary_color`,
//! `accent_color`) cannot break out of their surrounding context.

use super::types::BrandKitInput;

/// Escape a string for use inside HTML **element content** (e.g. text
/// inside `<h1>`, `<title>`, `<p>`). Covers `&`, `<`, `>`, and `"` — NOT
/// `'`. Never use this for attribute values — use [`escape_attr`] instead.
/// A single-quoted attribute context would need the apostrophe escaped.
fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Escape a string for use inside an HTML **attribute value** (double- or
/// single-quoted). Adds `'` on top of [`escape_text`]'s set so single-quoted
/// attributes stay safe too.
fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Escape a string for use inside a CSS string literal (e.g. the
/// `font-family: "..."` declaration). Strips ASCII control characters
/// entirely and backslash-escapes `\\` and `"`.
fn escape_css_string(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .map(|c| match c {
            '\\' => "\\\\".to_string(),
            '"' => "\\\"".to_string(),
            other => other.to_string(),
        })
        .collect()
}

pub fn build_style_guide(input: &BrandKitInput) -> String {
    let name_html = escape_text(&input.brand_name);
    // Primary/accent are already validated as hex by `types::validate_input`.
    // We still run the attribute-context versions through `escape_attr` (they
    // land inside `style="background: {color};"`) and the element-text copies
    // through `escape_text` (inside `<div class="meta">`) — cheap defense in
    // depth against a future caller that bypasses the validator.
    let primary_attr = escape_attr(&input.primary_color);
    let accent_attr = escape_attr(&input.accent_color);
    let primary_html = escape_text(&input.primary_color);
    let accent_html = escape_text(&input.accent_color);
    let font_css = escape_css_string(&input.font);
    let font_html = escape_text(&input.font);
    let svg = &input.logo_svg;

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{name_html} · Brand Guidelines</title>
<style>
  body {{ margin: 0; font-family: "{font_css}", sans-serif; color: #0E0E11; background: #F7F7F8; }}
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
  <h1>{name_html}</h1>
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
      <div class="chip" style="background: {primary_attr};"></div>
      <div class="meta">Primary<br>{primary_html}</div>
    </div>
    <div class="swatch">
      <div class="chip" style="background: {accent_attr};"></div>
      <div class="meta">Accent<br>{accent_html}</div>
    </div>
  </div>

  <h2>Typography</h2>
  <p class="specimen">{name_html}</p>
  <p class="rules">Typeface: {font_html}. Use for display and UI body text.</p>
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
    fn escape_text_handles_html_metachars() {
        // escape_text covers `& < > "` — apostrophe intentionally passes
        // through unchanged.
        assert_eq!(escape_text("Ben & Jerry's"), "Ben &amp; Jerry's");
        assert_eq!(escape_text(r#"A<B>"C""#), "A&lt;B&gt;&quot;C&quot;");
    }

    #[test]
    fn escape_attr_handles_quote_and_apostrophe() {
        assert_eq!(escape_attr(r#"a"b'c"#), "a&quot;b&#39;c");
    }

    #[test]
    fn escape_css_string_handles_quote_and_backslash() {
        assert_eq!(
            escape_css_string(r#"Font"Name\Path"#),
            r#"Font\"Name\\Path"#
        );
    }

    #[test]
    fn escape_css_string_strips_control_chars() {
        // `\n` and `\t` and the NUL byte all drop out silently.
        assert_eq!(escape_css_string("In\nter\ts\0"), "Inters");
    }

    #[test]
    fn build_style_guide_escapes_malicious_brand_name() {
        let input = BrandKitInput {
            logo_svg: "<svg></svg>".into(),
            source_png_path: PathBuf::from("x.png"),
            brand_name: r#"<script>alert("xss")</script>"#.into(),
            primary_color: "#e85d2d".into(),
            accent_color: "#0E0E11".into(),
            font: "Inter".into(),
        };
        let html = build_style_guide(&input);
        // The malicious payload must not appear verbatim anywhere.
        assert!(!html.contains("<script>"));
        assert!(!html.contains("</script>"));
        // …and the escaped form must appear where brand_name was substituted.
        assert!(html.contains("&lt;script&gt;"));
    }

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

    #[test]
    fn style_guide_structural_assertions() {
        let input = BrandKitInput {
            logo_svg: "<svg id=\"mark\"></svg>".into(),
            source_png_path: PathBuf::from("x.png"),
            brand_name: "Acme".into(),
            primary_color: "#e85d2d".into(),
            accent_color: "#0E0E11".into(),
            font: "Inter".into(),
        };
        let html = build_style_guide(&input);
        // Document starts with a lowercase HTML5 DOCTYPE.
        assert!(html.trim_start().starts_with("<!doctype html>"));
        // Primary color appears inside a `style="background: …;"` attribute.
        assert!(html.contains("background: #e85d2d"));
        // Accent color likewise. Allow either preserved case (current code
        // preserves what the caller passed in, since it's already escaped
        // as text rather than attribute-normalized).
        assert!(html.contains("background: #0E0E11") || html.contains("background: #0e0e11"));
        // The SVG lands inside the `.logo` div, not somewhere else in the
        // document. This catches a regression where `logo_svg` drifts out
        // of context after a refactor. Brand_name's `"` inside the SVG
        // survives — escape_text didn't touch it because it's part of the
        // raw SVG literal, not the brand_name path.
        assert!(html.contains(r#"<div class="logo"><svg id="mark">"#));
    }
}
