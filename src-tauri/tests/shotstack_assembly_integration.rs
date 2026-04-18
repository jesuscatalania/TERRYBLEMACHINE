//! End-to-end tests for [`ShotstackAssembler`].
//!
//! Unlike `mesh_pipeline_integration.rs` / `video_pipeline_integration.rs`
//! which build a fake `AiClient` and drive the pipeline through an
//! `AiRouter`, `ShotstackAssembler` owns a real `ShotstackClient` directly
//! (no routing — see the module docstring for why). So we spin up a
//! `wiremock::MockServer` that speaks Shotstack's wire protocol (POST
//! `/edit/stage/render` → render id, GET `/edit/stage/render/{id}` → `done`
//! with a `file://…` URL) and let the assembler pull the full submit → poll
//! → download cycle through it. The `file://` special-case inside
//! `download_to_cache` lets us verify the full pipeline without a second
//! mock HTTP server to serve the MP4 bytes.

use std::sync::Arc;

use serde_json::json;
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use terryblemachine_lib::api_clients::shotstack::{
    ShotstackClient, ShotstackEnv, KEYCHAIN_SERVICE,
};

/// Build a `ShotstackClient` pointed at the wiremock `base_url`. Integration
/// tests can't reach `#[cfg(test)] for_test`, so we use the public
/// `with_base_url` and accept the default-rate-limited bucket (5 rps, plenty
/// for the in-process wiremock). Defaults to the Stage env — integration
/// tests exercise the sandbox render path.
fn test_client(store: Arc<dyn KeyStore>, base_url: String) -> Arc<ShotstackClient> {
    Arc::new(ShotstackClient::with_base_url(
        store,
        base_url,
        5,
        ShotstackEnv::Stage,
    ))
}
use terryblemachine_lib::keychain::{InMemoryStore, KeyStore};
use terryblemachine_lib::shotstack_assembly::{
    AssemblyClip, AssemblyError, AssemblyInput, ShotstackAssembler, StubAssembler, VideoAssembler,
};

fn key_store_with_key() -> Arc<dyn KeyStore> {
    let s = InMemoryStore::new();
    s.store(KEYCHAIN_SERVICE, "sk-test").unwrap();
    Arc::new(s)
}

fn clip(src: &str, start: f32, length: f32) -> AssemblyClip {
    AssemblyClip {
        src: src.into(),
        start_s: start,
        length_s: length,
        transition_in: None,
        transition_out: None,
    }
}

fn assembly_input(clips: Vec<AssemblyClip>) -> AssemblyInput {
    AssemblyInput {
        clips,
        soundtrack: None,
        format: "mp4".into(),
        resolution: "hd".into(),
    }
}

// ─── ShotstackAssembler (production pipeline) ──────────────────────────

#[tokio::test]
async fn assemble_downloads_rendered_mp4_to_cache() {
    let tmp = TempDir::new().unwrap();
    let fake_mp4 = tmp.path().join("fake.mp4");
    // Magic bytes don't matter — the cache step is a byte-for-byte copy.
    std::fs::write(&fake_mp4, b"ftyp\x00\x00\x00\x00fake-mp4").unwrap();
    let file_url = format!("file://{}", fake_mp4.display());

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/edit/stage/render"))
        .and(header("x-api-key", "sk-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "response": { "id": "render-int-1" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/edit/stage/render/render-int-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "response": { "status": "done", "url": file_url }
        })))
        .mount(&server)
        .await;

    let client = test_client(key_store_with_key(), server.uri());
    let assembler = ShotstackAssembler::new(client);

    let result = assembler
        .assemble(assembly_input(vec![
            clip("https://cdn/a.mp4", 0.0, 2.0),
            clip("https://cdn/b.mp4", 2.0, 3.0),
        ]))
        .await
        .expect("assembly succeeds");

    assert_eq!(result.render_id, "render-int-1");
    assert!(
        result.video_url.contains("fake.mp4"),
        "video_url should echo Shotstack done URL, got {}",
        result.video_url
    );
    let local = result
        .local_path
        .expect("cache path present after download");
    assert!(local.exists(), "cached MP4 should exist at {local:?}");
    assert!(
        local.extension().map(|e| e == "mp4").unwrap_or(false),
        "cache path should end in .mp4, got {local:?}"
    );
    // Byte-for-byte content match — the cache copy preserved payload.
    assert_eq!(
        std::fs::read(&local).unwrap(),
        b"ftyp\x00\x00\x00\x00fake-mp4"
    );
}

#[tokio::test]
async fn assemble_rejects_empty_clips_before_hitting_shotstack() {
    // No mock endpoints — if the pipeline tried to POST, we'd crash with
    // a connection error (which would also be a pass). A clean
    // `InvalidInput` return value is stricter.
    let server = MockServer::start().await;
    let client = test_client(key_store_with_key(), server.uri());
    let assembler = ShotstackAssembler::new(client);

    let err = assembler
        .assemble(assembly_input(vec![]))
        .await
        .expect_err("empty clips must be rejected");
    assert!(
        matches!(err, AssemblyError::InvalidInput(_)),
        "expected InvalidInput, got {err:?}"
    );
}

#[tokio::test]
async fn assemble_bubbles_shotstack_failure_as_provider_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/edit/stage/render"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "response": { "id": "render-fail-1" }
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/edit/stage/render/render-fail-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "response": { "status": "failed", "error": "bad asset" }
        })))
        .mount(&server)
        .await;

    let client = test_client(key_store_with_key(), server.uri());
    let assembler = ShotstackAssembler::new(client);

    let err = assembler
        .assemble(assembly_input(vec![clip("https://cdn/a.mp4", 0.0, 1.0)]))
        .await
        .expect_err("failed render must bubble as Provider error");
    match err {
        AssemblyError::Provider(m) => assert!(
            m.contains("bad asset"),
            "provider error should bubble Shotstack message, got: {m}"
        ),
        other => panic!("wrong error variant: {other:?}"),
    }
}

#[tokio::test]
async fn assemble_download_is_idempotent_across_calls() {
    let tmp = TempDir::new().unwrap();
    let fake_mp4 = tmp.path().join("twice.mp4");
    std::fs::write(&fake_mp4, b"ftyp").unwrap();
    let file_url = format!("file://{}", fake_mp4.display());

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/edit/stage/render"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "response": { "id": "render-idem-1" }
        })))
        .mount(&server)
        .await;

    // Both calls hit the same render id → same URL → same cache path.
    Mock::given(method("GET"))
        .and(path("/edit/stage/render/render-idem-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "response": { "status": "done", "url": file_url }
        })))
        .mount(&server)
        .await;

    let client = test_client(key_store_with_key(), server.uri());
    let assembler = ShotstackAssembler::new(client);

    let r1 = assembler
        .assemble(assembly_input(vec![clip("x", 0.0, 1.0)]))
        .await
        .expect("first call");
    let r2 = assembler
        .assemble(assembly_input(vec![clip("x", 0.0, 1.0)]))
        .await
        .expect("second call");

    assert_eq!(
        r1.local_path, r2.local_path,
        "identical remote URL → identical cache path"
    );
}

// ─── StubAssembler ─────────────────────────────────────────────────────
//
// The stub lives in-crate and has its own unit tests, but we also exercise
// it through the same `VideoAssembler` trait the commands layer sees.
// Belt-and-braces: catches any divergence between stub and production that
// would break the IPC contract.

#[tokio::test]
async fn stub_assembler_returns_deterministic_result() {
    let a = StubAssembler::new();
    let r = a
        .assemble(assembly_input(vec![
            clip("a", 0.0, 1.0),
            clip("b", 1.0, 1.0),
            clip("c", 2.0, 1.0),
        ]))
        .await
        .unwrap();
    assert_eq!(r.render_id, "stub-render-id");
    assert_eq!(r.video_url, "stub://assembly/3.mp4");
    assert!(r.local_path.is_none());
}

#[tokio::test]
async fn stub_assembler_rejects_empty_clips() {
    let a = StubAssembler::new();
    let err = a.assemble(assembly_input(vec![])).await.unwrap_err();
    assert!(matches!(err, AssemblyError::InvalidInput(_)));
}
