//! Real end-to-end test for the Playwright URL analyzer.
//!
//! The `example.com` test is ignored by default — requires `node` + a built
//! `scripts/url_analyzer.mjs` + network access. Run manually with:
//!
//! ```sh
//! cargo test --test website_analyzer_e2e -- --ignored
//! ```

use std::path::PathBuf;

use terryblemachine_lib::website_analyzer::commands::{resolve_assets_dir, AnalyzerIpcError};
use terryblemachine_lib::website_analyzer::{PlaywrightUrlAnalyzer, UrlAnalyzer};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

/// Minimal valid 16x16 transparent PNG (for favicon fixture).
fn tiny_png() -> Vec<u8> {
    // 1x1 PNG with a single transparent pixel — byte-for-byte canonical form.
    // Source: https://github.com/mathiasbynens/small
    const DATA: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    DATA.to_vec()
}

/// End-to-end: serves a tiny HTML page with an `<img>` and a favicon via
/// `wiremock::MockServer`, runs the real Playwright sidecar against it with
/// `assets_dir = Some(...)`, and asserts that `downloadAssets()` actually ran
/// — i.e. the assets directory contains at least one file and the returned
/// `AnalysisResult.assets` vec is non-empty.
///
/// Marked `#[ignore]` because it needs the real Chromium browser that
/// `playwright` downloads on first `npx playwright install`. In CI sandboxes
/// that bar network egress during the cargo test step this cannot succeed.
///
/// Run locally with:
///
/// ```sh
/// cargo test --test website_analyzer_e2e analyze_downloads_assets_from_mock_server -- --ignored
/// ```
///
/// (FU #100 — the only test that exercises the 90-line `downloadAssets()`
/// function in `scripts/url_analyzer.mjs`.)
#[tokio::test]
#[ignore]
async fn analyze_downloads_assets_from_mock_server() {
    let server = MockServer::start().await;

    let html = r#"<!doctype html>
<html><head>
  <title>FU100 Fixture</title>
  <link rel="icon" href="/favicon.ico">
</head><body>
  <img src="/pic.png" alt="pic">
</body></html>"#;

    // NOTE: wiremock's `set_body_string` hard-codes `Content-Type: text/plain`
    // and `insert_header` is additive, so we must use `set_body_raw` to get
    // the browser to parse the response as HTML. Without this, Chromium
    // treats the body as plain text and no `<img>`/`<link>` tags land in the
    // DOM — which silently breaks the asset-download assertion.
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(html.as_bytes(), "text/html"))
        .mount(&server)
        .await;

    let png = tiny_png();
    Mock::given(method("GET"))
        .and(path("/favicon.ico"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(png.clone(), "image/png"))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/pic.png"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(png, "image/png"))
        .mount(&server)
        .await;

    let assets_dir = tempfile::tempdir().expect("tempdir");

    let analyzer = PlaywrightUrlAnalyzer::new(script_path());
    let result = analyzer
        .analyze(&server.uri(), None, Some(assets_dir.path()))
        .await
        .expect("analyzer should succeed against mock server");

    assert_eq!(result.status, 200);
    assert_eq!(result.title, "FU100 Fixture");
    assert!(
        !result.assets.is_empty(),
        "expected at least one downloaded asset entry in AnalysisResult, got: {:?}",
        result.assets
    );

    // Directory must contain at least one saved file — the `saved_as` names
    // in `result.assets` are relative to the assets directory.
    let mut on_disk = 0;
    let mut entries = tokio::fs::read_dir(assets_dir.path())
        .await
        .expect("read assets dir");
    while let Ok(Some(_)) = entries.next_entry().await {
        on_disk += 1;
    }
    assert!(
        on_disk > 0,
        "expected at least one file saved under {}, got {on_disk}",
        assets_dir.path().display()
    );
}

// ─── FU #101: project_path canonicalization ───────────────────────────────

/// Happy path: a project directory that lives directly under the projects
/// root must be accepted, and the returned `assets_dir` must point at
/// `<canonical project>/assets`.
#[test]
fn resolve_assets_dir_accepts_project_inside_root() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("my-site");
    std::fs::create_dir_all(&project).unwrap();

    let assets = resolve_assets_dir(&project, root.path()).expect("project under root accepted");

    let canon_project = std::fs::canonicalize(&project).unwrap();
    assert_eq!(assets, canon_project.join("assets"));
}

/// Symlink-traversal guard: a `project_path` whose canonical form lies
/// *outside* the trusted projects root must be rejected with
/// `InvalidRequest`. Without this check, a `<root>/escape → /etc` symlink
/// would let the sidecar download assets into arbitrary OS directories.
#[cfg(unix)]
#[test]
fn resolve_assets_dir_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();

    // Create a symlink inside the root that points at a directory outside it.
    let escape = root.path().join("escape");
    symlink(elsewhere.path(), &escape).expect("create symlink");

    let err = resolve_assets_dir(&escape, root.path()).expect_err("symlink escape must be denied");
    match err {
        AnalyzerIpcError::InvalidRequest(msg) => {
            assert!(
                msg.contains("must be under projects_root"),
                "unexpected message: {msg}"
            );
        }
        other => panic!("expected InvalidRequest, got {other:?}"),
    }
}

/// A project_path that references a sibling directory (`../other`) must be
/// rejected even though the literal string starts with the root prefix.
#[test]
fn resolve_assets_dir_rejects_sibling_directory() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("projects");
    let sibling = parent.path().join("other-projects");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&sibling).unwrap();

    let err = resolve_assets_dir(&sibling, &root).expect_err("sibling must be denied");
    assert!(matches!(err, AnalyzerIpcError::InvalidRequest(_)));
}

/// A project_path that does not exist on disk must surface as
/// `InvalidRequest` (canonicalize fails before we even get to the
/// starts_with check).
#[test]
fn resolve_assets_dir_rejects_missing_path() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join("nope");

    let err = resolve_assets_dir(&missing, root.path()).expect_err("missing path must error");
    match err {
        AnalyzerIpcError::InvalidRequest(msg) => {
            assert!(msg.contains("project_path"), "unexpected message: {msg}");
        }
        other => panic!("expected InvalidRequest, got {other:?}"),
    }
}
