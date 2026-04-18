//! Integration tests for the Remotion render pipeline.
//!
//! These tests cover validation guards (empty composition, injection
//! characters) without actually spawning Remotion. A happy-path render
//! test would require Chromium + the remotion/ subpackage installed,
//! so it's not part of the automated suite — verified manually.

use serde_json::json;
use tempfile::TempDir;
use terryblemachine_lib::remotion::{commands::render_inner, RemotionError, RemotionInput};

#[tokio::test]
async fn render_rejects_empty_composition() {
    let tmp = TempDir::new().unwrap();
    let err = render_inner(
        tmp.path(),
        &RemotionInput {
            composition: "   ".into(),
            props: json!({}),
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, RemotionError::InvalidInput(_)));
}

#[tokio::test]
async fn render_rejects_injection_characters() {
    let tmp = TempDir::new().unwrap();
    let err = render_inner(
        tmp.path(),
        &RemotionInput {
            composition: "A; rm -rf /".into(),
            props: json!({}),
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, RemotionError::InvalidInput(_)));
}

#[tokio::test]
async fn render_rejects_slashes_in_composition() {
    let tmp = TempDir::new().unwrap();
    let err = render_inner(
        tmp.path(),
        &RemotionInput {
            composition: "../etc/passwd".into(),
            props: json!({}),
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, RemotionError::InvalidInput(_)));
}

#[tokio::test]
async fn render_accepts_valid_alphanumeric_composition_name() {
    // This test will invoke npx but fail at the actual Remotion step
    // because the tempdir isn't a remotion subpackage. We're only
    // asserting the validation passes — the process error is expected.
    let tmp = TempDir::new().unwrap();
    let result = render_inner(
        tmp.path(),
        &RemotionInput {
            composition: "ValidName_123".into(),
            props: json!({ "text": "ok" }),
        },
    )
    .await;
    // Expect Process error (npx/remotion can't actually run here),
    // NOT an InvalidInput error from the validation.
    match result {
        Err(RemotionError::Process(_)) => {} // expected
        Err(RemotionError::Cache(_)) => {}   // also acceptable in CI sandbox
        other => panic!("expected Process or Cache error, got: {other:?}"),
    }
}
