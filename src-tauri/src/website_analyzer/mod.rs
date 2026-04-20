//! URL analyzer — extracts colors / fonts / layout from any public website.
//!
//! The heavy lifting runs in a Node sidecar (`scripts/url_analyzer.mjs`) via
//! Playwright. Rust spawns the script, parses its single-line JSON output,
//! and returns an [`AnalysisResult`].
//!
//! For unit tests we use a trait-based [`UrlAnalyzer`] abstraction + a
//! [`StubUrlAnalyzer`] test double so nothing depends on a running browser.

pub mod commands;
mod playwright;
mod stub;
mod types;

pub use playwright::PlaywrightUrlAnalyzer;
pub use stub::StubUrlAnalyzer;
pub use types::{
    AnalysisResult, AnalyzerError, AssetDownload, ColorRoles, DetectedFeatures, TypographyStyle,
    UrlAnalyzer,
};
