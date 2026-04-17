//! Real end-to-end test for the Playwright URL analyzer.
//!
//! Ignored by default — requires `node` + a built `scripts/url_analyzer.mjs`
//! + network access to `example.com`. Run manually with:
//!
//! ```sh
//! cargo test --test website_analyzer_e2e -- --ignored
//! ```

use std::path::PathBuf;

use terryblemachine_lib::website_analyzer::{PlaywrightUrlAnalyzer, UrlAnalyzer};

fn script_path() -> PathBuf {
    // `cargo test` runs from src-tauri/ — script lives in the workspace root.
    PathBuf::from("..").join("scripts").join("url_analyzer.mjs")
}

#[tokio::test]
#[ignore]
async fn analyze_example_com_returns_populated_result() {
    let analyzer = PlaywrightUrlAnalyzer::new(script_path());
    let result = analyzer
        .analyze("https://example.com", None)
        .await
        .expect("analyzer should succeed against example.com");

    assert_eq!(result.url, "https://example.com");
    assert_eq!(result.status, 200);
    assert_eq!(result.title, "Example Domain");
    assert!(!result.colors.is_empty(), "expected at least one colour");
    assert!(!result.fonts.is_empty(), "expected at least one font");
    // Example.com is simple — layout should be classified as "other".
    assert!(["grid", "flex", "other"].contains(&result.layout.as_str()));
}
