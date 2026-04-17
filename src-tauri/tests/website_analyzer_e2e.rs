//! Real end-to-end test for the Playwright URL analyzer.
//!
//! The `example.com` test is ignored by default — requires `node` + a built
//! `scripts/url_analyzer.mjs` + network access. Run manually with:
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
        .analyze("https://example.com", None, None)
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

/// Verifies `PlaywrightUrlAnalyzer::analyze` forwards `--assets-dir=<path>`
/// to the sidecar when `assets_dir` is Some. Swaps the "node" binary for a
/// shell script that records its argv to a file and emits a valid AnalysisResult.
#[cfg(unix)]
#[tokio::test]
async fn analyze_passes_assets_dir_flag_to_sidecar() {
    use std::os::unix::fs::PermissionsExt;
    use tokio::fs;

    let tmp = tempfile::tempdir().expect("tempdir");
    let argv_log = tmp.path().join("argv.txt");
    let fake_node = tmp.path().join("fake_node.sh");
    let assets_dir = tmp.path().join("assets");

    // Shell script: logs argv, prints a valid AnalysisResult JSON on stdout.
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > {log}\n\
         printf '{{\"url\":\"https://x\",\"status\":200,\"title\":\"X\",\"colors\":[],\"fonts\":[],\"spacing\":[],\"customProperties\":{{}},\"layout\":\"flex\",\"assets\":[]}}\\n'\n",
        log = argv_log.display()
    );
    fs::write(&fake_node, script)
        .await
        .expect("write fake node");
    let mut perms = fs::metadata(&fake_node).await.unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&fake_node, perms).await.unwrap();

    // Arbitrary script path — the fake "node" ignores it after logging.
    let analyzer = PlaywrightUrlAnalyzer::new(PathBuf::from("scripts/url_analyzer.mjs")).with_node(
        fake_node
            .to_str()
            .expect("fake_node path is valid utf-8")
            .to_string(),
    );

    let result = analyzer
        .analyze("https://x", None, Some(assets_dir.as_path()))
        .await
        .expect("analyzer should succeed with fake sidecar");
    assert_eq!(result.url, "https://x");

    let logged = fs::read_to_string(&argv_log).await.expect("read argv log");
    let flag = format!("--assets-dir={}", assets_dir.display());
    assert!(
        logged.lines().any(|l| l == flag),
        "expected `{flag}` in argv, got:\n{logged}"
    );
}
