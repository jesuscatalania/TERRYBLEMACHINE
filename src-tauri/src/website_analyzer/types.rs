use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One URL analysis pass — a flattened summary of everything the Playwright
/// sidecar extracted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub url: String,
    /// HTTP status the sidecar observed (0 if unknown).
    #[serde(default)]
    pub status: u16,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Top dominant colours, most-frequent-first. Values are the raw
    /// `rgb(...)` / `rgba(...)` strings reported by `getComputedStyle`.
    pub colors: Vec<String>,
    /// Distinct `font-family` primary values observed.
    pub fonts: Vec<String>,
    /// Most-common non-zero spacing values (padding/margin).
    pub spacing: Vec<String>,
    /// CSS custom properties declared on `:root` (e.g. `--primary`).
    #[serde(rename = "customProperties")]
    pub custom_properties: HashMap<String, String>,
    /// Coarse layout classification — "grid" / "flex" / "other".
    pub layout: String,
    /// Absolute path to the saved screenshot, when requested.
    #[serde(
        rename = "screenshotPath",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub screenshot_path: Option<PathBuf>,
    /// Assets downloaded into the caller-supplied assets directory. Empty
    /// when no `assets_dir` was provided to the analyzer. The filenames in
    /// `saved_as` are relative to that directory.
    #[serde(default)]
    pub assets: Vec<AssetDownload>,

    // ── Deep-analysis fields (additive, serde(default) for backward-compat
    //    with older cached analyses that predate these signals).
    /// First `<h1>` on the page, trimmed to 200 chars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hero_text: Option<String>,
    /// Up to 8 `<nav>`/`<header>` anchor labels.
    #[serde(default)]
    pub nav_items: Vec<String>,
    /// Up to 12 `<h2>` texts, each capped at 140 chars.
    #[serde(default)]
    pub section_headings: Vec<String>,
    /// First 2-3 visible `<p>` excerpts (≤300 chars each).
    #[serde(default)]
    pub paragraph_sample: Vec<String>,
    /// Up to 6 CTA labels (`<button>` / `<a class~=button>`).
    #[serde(default)]
    pub cta_labels: Vec<String>,
    /// Feature-detection booleans.
    #[serde(default)]
    pub detected_features: DetectedFeatures,
    /// Up to 4 most-used (size, weight, family) combos.
    #[serde(default)]
    pub typography: Vec<TypographyStyle>,
    /// Up to 12 `<img src>` absolute URLs.
    #[serde(default)]
    pub image_urls: Vec<String>,
    /// Semantic color roles extracted from `<body>` computed style +
    /// most-seen non-bg/fg accent.
    #[serde(default)]
    pub color_roles: ColorRoles,
}

/// Features detected on the page (presence of specific DOM/JS signals).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DetectedFeatures {
    #[serde(default)]
    pub has_canvas: bool,
    #[serde(default)]
    pub has_video: bool,
    #[serde(default)]
    pub has_form: bool,
    #[serde(default)]
    pub has_iframe: bool,
    #[serde(default)]
    pub has_webgl: bool,
    #[serde(default)]
    pub has_three_js: bool,
}

/// One (size, weight, family) typography combo — as observed in the DOM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypographyStyle {
    pub size: String,
    pub weight: String,
    pub family: String,
}

/// Semantic color roles — bg/fg pulled from `<body>`, accent = most-seen
/// non-bg/fg color from the dominant palette.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ColorRoles {
    #[serde(default)]
    pub bg: Option<String>,
    #[serde(default)]
    pub fg: Option<String>,
    #[serde(default)]
    pub accent: Option<String>,
}

/// One downloaded asset — the original remote URL + the filename the sidecar
/// wrote into the assets directory (relative to that directory).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetDownload {
    pub url: String,
    pub saved_as: String,
}

#[derive(Debug, Error)]
pub enum AnalyzerError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("sidecar spawn failed: {0}")]
    Spawn(String),

    #[error("sidecar exited with error: {0}")]
    Sidecar(String),

    #[error("malformed analyzer output: {0}")]
    ParseOutput(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[async_trait]
pub trait UrlAnalyzer: Send + Sync {
    /// Analyze `url`.
    ///
    /// - `screenshot_path`: when set, the sidecar saves a page screenshot there.
    /// - `assets_dir`:      when set, referenced images / icons / fonts are
    ///                      downloaded into that directory.
    async fn analyze(
        &self,
        url: &str,
        screenshot_path: Option<&std::path::Path>,
        assets_dir: Option<&std::path::Path>,
    ) -> Result<AnalysisResult, AnalyzerError>;
}
