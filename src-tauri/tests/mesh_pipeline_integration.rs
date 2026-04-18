//! End-to-end tests for [`RouterMeshPipeline`].
//!
//! Mirrors `depth_pipeline_integration.rs` but exercises the GLB-download
//! step: we write a tiny fake-GLB to a tempdir, build a fake Meshy client
//! that echoes back a `file://…` URL pointing at it, and let
//! `RouterMeshPipeline` pull it through the router → download path → cache
//! dir. The `file://` special-case inside `download_to_cache` lets us
//! verify the full pipeline without a mock HTTP server.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::json;
use tempfile::TempDir;

use terryblemachine_lib::ai_router::{
    AiClient, AiRequest, AiResponse, AiRouter, Complexity, DefaultRoutingStrategy, Model,
    PriorityQueue, Provider, ProviderError, ProviderUsage, RetryPolicy,
};
use terryblemachine_lib::mesh_pipeline::commands::{export_mesh_inner, MeshIpcError};
use terryblemachine_lib::mesh_pipeline::{
    MeshImageInput, MeshPipeline, MeshPipelineError, MeshTextInput, RouterMeshPipeline,
};

/// Fake Meshy client that echoes a predetermined `glb_url` back inside the
/// router response. We use a `file://` URL so the pipeline's download step
/// stays local to this test. Captures the most recent `AiRequest` so tests
/// can assert routing inputs (complexity, task, payload, …).
struct StubMeshyClient {
    glb_url_to_echo: String,
    last_request: Arc<Mutex<Option<AiRequest>>>,
}

#[async_trait]
impl AiClient for StubMeshyClient {
    fn provider(&self) -> Provider {
        Provider::Meshy
    }

    fn supports(&self, m: Model) -> bool {
        matches!(m, Model::MeshyText3D | Model::MeshyImage3D)
    }

    async fn execute(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        *self.last_request.lock().unwrap() = Some(request.clone());
        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({
                "glb_url": self.glb_url_to_echo,
                "status": "succeeded",
            }),
            cost_cents: None,
            cached: false,
        })
    }

    async fn health_check(&self) -> bool {
        true
    }

    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
        Ok(ProviderUsage::default())
    }
}

/// Fake Replicate client that only supports TripoSR and echoes a
/// predetermined GLB URL. Captures the most recent `AiRequest` so tests can
/// assert `Complexity::Simple` actually reached the client.
struct StubReplicateClient {
    glb_url_to_echo: String,
    last_request: Arc<Mutex<Option<AiRequest>>>,
}

#[async_trait]
impl AiClient for StubReplicateClient {
    fn provider(&self) -> Provider {
        Provider::Replicate
    }

    fn supports(&self, m: Model) -> bool {
        matches!(m, Model::ReplicateTripoSR)
    }

    async fn execute(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        *self.last_request.lock().unwrap() = Some(request.clone());
        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({
                "glb_url": self.glb_url_to_echo,
                "status": "succeeded",
            }),
            cost_cents: None,
            cached: false,
        })
    }

    async fn health_check(&self) -> bool {
        true
    }

    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
        Ok(ProviderUsage::default())
    }
}

fn pipeline_with_glb(tmp: &TempDir) -> (RouterMeshPipeline, std::path::PathBuf) {
    let (p, path, _meshy_capture, _replicate_capture) = pipeline_with_capture(tmp);
    (p, path)
}

/// Build the pipeline with both a Meshy stub and a Replicate (TripoSR) stub
/// and return the Mutex handles so tests can assert what the router actually
/// dispatched to each provider.
#[allow(clippy::type_complexity)]
fn pipeline_with_capture(
    tmp: &TempDir,
) -> (
    RouterMeshPipeline,
    std::path::PathBuf,
    Arc<Mutex<Option<AiRequest>>>,
    Arc<Mutex<Option<AiRequest>>>,
) {
    let fake_glb = tmp.path().join("fake.glb");
    // Minimal well-formed GLB magic — test asserts cache round-trip, not parse.
    std::fs::write(&fake_glb, b"glTF\x02\x00\x00\x00").unwrap();
    let file_url = format!("file://{}", fake_glb.display());

    let meshy_capture: Arc<Mutex<Option<AiRequest>>> = Arc::new(Mutex::new(None));
    let replicate_capture: Arc<Mutex<Option<AiRequest>>> = Arc::new(Mutex::new(None));

    let mut clients: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();
    clients.insert(
        Provider::Meshy,
        Arc::new(StubMeshyClient {
            glb_url_to_echo: file_url.clone(),
            last_request: Arc::clone(&meshy_capture),
        }),
    );
    clients.insert(
        Provider::Replicate,
        Arc::new(StubReplicateClient {
            glb_url_to_echo: file_url,
            last_request: Arc::clone(&replicate_capture),
        }),
    );
    let router = Arc::new(AiRouter::new(
        Arc::new(DefaultRoutingStrategy),
        clients,
        RetryPolicy::default_policy(),
        Arc::new(PriorityQueue::new()),
    ));
    (
        RouterMeshPipeline::new(router),
        fake_glb,
        meshy_capture,
        replicate_capture,
    )
}

#[tokio::test]
async fn text_to_mesh_downloads_to_cache() {
    let tmp = TempDir::new().unwrap();
    let (p, _src) = pipeline_with_glb(&tmp);

    let r = p
        .generate_from_text(MeshTextInput {
            prompt: "a cup".into(),
            module: None,
        })
        .await
        .expect("text-to-mesh succeeds");

    assert!(
        r.glb_url.contains("fake.glb"),
        "glb_url should echo stub URL, got {}",
        r.glb_url
    );
    let local = r.local_path.expect("cache path present after download");
    assert!(local.exists(), "cached GLB file should exist at {local:?}");
    assert!(
        local.extension().map(|e| e == "glb").unwrap_or(false),
        "cache path should end in .glb, got {local:?}"
    );
    assert_eq!(r.model, format!("{:?}", Model::MeshyText3D));
}

#[tokio::test]
async fn text_to_mesh_rejects_empty_prompt() {
    let tmp = TempDir::new().unwrap();
    let (p, _) = pipeline_with_glb(&tmp);

    let err = p
        .generate_from_text(MeshTextInput {
            prompt: "   ".into(),
            module: None,
        })
        .await
        .expect_err("empty prompt must be rejected before routing");

    assert!(
        matches!(err, MeshPipelineError::InvalidInput(_)),
        "expected InvalidInput, got {err:?}"
    );
}

#[tokio::test]
async fn image_to_mesh_rejects_data_url() {
    let tmp = TempDir::new().unwrap();
    let (p, _) = pipeline_with_glb(&tmp);

    let err = p
        .generate_from_image(MeshImageInput {
            image_url: "data:image/png;base64,abc".into(),
            prompt: None,
            module: None,
            quick_preview: false,
        })
        .await
        .expect_err("data-URL must be rejected before routing");

    assert!(
        matches!(err, MeshPipelineError::InvalidInput(_)),
        "expected InvalidInput, got {err:?}"
    );
    assert!(
        format!("{err}").contains("data-URLs"),
        "error should mention data-URLs, got: {err}"
    );
}

#[tokio::test]
async fn download_is_idempotent_across_calls() {
    let tmp = TempDir::new().unwrap();
    let (p, _) = pipeline_with_glb(&tmp);

    let r1 = p
        .generate_from_text(MeshTextInput {
            prompt: "cube".into(),
            module: None,
        })
        .await
        .expect("first call succeeds");
    let r2 = p
        .generate_from_text(MeshTextInput {
            prompt: "cube".into(),
            module: None,
        })
        .await
        .expect("second call succeeds");

    // Both responses echo the same remote URL, so the content-addressed
    // cache path must match byte-for-byte.
    assert_eq!(
        r1.local_path, r2.local_path,
        "identical remote URL → identical cache path"
    );
}

#[tokio::test]
async fn image_to_mesh_default_routes_via_meshy_with_medium_complexity() {
    let tmp = TempDir::new().unwrap();
    let (p, _, meshy_capture, replicate_capture) = pipeline_with_capture(&tmp);

    let r = p
        .generate_from_image(MeshImageInput {
            image_url: "https://hosted.example/a.png".into(),
            prompt: None,
            module: None,
            quick_preview: false,
        })
        .await
        .expect("image-to-mesh succeeds");

    assert_eq!(r.model, format!("{:?}", Model::MeshyImage3D));
    let captured = meshy_capture
        .lock()
        .unwrap()
        .clone()
        .expect("meshy stub must have been invoked");
    assert_eq!(captured.complexity, Complexity::Medium);
    assert!(
        replicate_capture.lock().unwrap().is_none(),
        "replicate must not be touched on default routing"
    );
}

#[tokio::test]
async fn image_to_mesh_quick_preview_routes_via_simple_complexity() {
    let tmp = TempDir::new().unwrap();
    let (p, _, meshy_capture, replicate_capture) = pipeline_with_capture(&tmp);

    let r = p
        .generate_from_image(MeshImageInput {
            image_url: "https://hosted.example/a.png".into(),
            prompt: None,
            module: None,
            quick_preview: true,
        })
        .await
        .expect("quick-preview image-to-mesh succeeds");

    assert_eq!(
        r.model,
        format!("{:?}", Model::ReplicateTripoSR),
        "quick_preview must dispatch to TripoSR"
    );
    let captured = replicate_capture
        .lock()
        .unwrap()
        .clone()
        .expect("replicate stub must have been invoked");
    assert_eq!(captured.complexity, Complexity::Simple);
    assert!(
        meshy_capture.lock().unwrap().is_none(),
        "meshy must not be touched when TripoSR succeeds"
    );
}

// ─── export_mesh ────────────────────────────────────────────────────────
//
// `export_mesh_inner` is the helper the `#[tauri::command]` wraps — the tests
// below drive it directly so we can pass a plain `projects_root` without
// building a Tauri `State<ProjectStoreState>`. Its job is filesystem-bound
// plus a path-traversal guard that mirrors FU #101's `resolve_assets_dir`.

#[test]
fn export_mesh_copies_cached_glb_to_target() {
    let tmp = TempDir::new().unwrap();
    let projects_root = tmp.path();
    let src = projects_root.join("cached.glb");
    std::fs::write(&src, b"glTF\x02\x00\x00\x00test").unwrap();
    let target = projects_root.join("exports/out.glb");

    let result = export_mesh_inner(projects_root, &src, &target).expect("export ok");
    assert_eq!(result, target);
    assert!(target.exists());
    assert_eq!(std::fs::read(&target).unwrap(), b"glTF\x02\x00\x00\x00test");
}

#[test]
fn export_mesh_creates_parent_dir() {
    let tmp = TempDir::new().unwrap();
    let projects_root = tmp.path();
    let src = projects_root.join("cached.glb");
    std::fs::write(&src, b"GLB").unwrap();
    let target = projects_root.join("nested/dir/chain/out.glb");

    export_mesh_inner(projects_root, &src, &target).expect("export ok");
    assert!(target.exists());
}

#[test]
fn export_mesh_rejects_missing_source() {
    let tmp = TempDir::new().unwrap();
    let projects_root = tmp.path();
    let src = projects_root.join("does-not-exist.glb");
    let target = projects_root.join("out.glb");

    let err = export_mesh_inner(projects_root, &src, &target).expect_err("should fail");
    match err {
        MeshIpcError::InvalidInput(msg) => assert!(msg.contains("not in cache")),
        other => panic!("wrong error variant: {other:?}"),
    }
}

#[test]
fn export_mesh_rejects_target_outside_projects_root() {
    let tmp = TempDir::new().unwrap();
    let projects_root = tmp.path().join("projects");
    std::fs::create_dir_all(&projects_root).unwrap();
    let src = projects_root.join("cached.glb");
    std::fs::write(&src, b"GLB").unwrap();
    // Target lives outside projects_root — lexical check must reject it.
    let outside = tmp.path().join("outside/out.glb");

    let err = export_mesh_inner(&projects_root, &src, &outside).expect_err("should fail");
    match err {
        MeshIpcError::InvalidInput(msg) => assert!(
            msg.contains("must be under projects_root"),
            "unexpected msg: {msg}"
        ),
        other => panic!("wrong error variant: {other:?}"),
    }
    // Must NOT have created the outside directory tree.
    assert!(!outside.parent().unwrap().exists());
}

#[cfg(unix)]
#[test]
fn export_mesh_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;

    let tmp = TempDir::new().unwrap();
    let projects_root = tmp.path().join("projects");
    std::fs::create_dir_all(&projects_root).unwrap();
    let escape_target = tmp.path().join("escape");
    std::fs::create_dir_all(&escape_target).unwrap();

    let src = projects_root.join("cached.glb");
    std::fs::write(&src, b"GLB").unwrap();

    // <projects_root>/link → <tmp>/escape (outside projects_root).
    // The lexical check passes because the path string starts with
    // projects_root; only canonicalize catches the symlink escape.
    let link = projects_root.join("link");
    symlink(&escape_target, &link).unwrap();
    let target = link.join("out.glb");

    let err = export_mesh_inner(&projects_root, &src, &target).expect_err("should fail");
    match err {
        MeshIpcError::InvalidInput(msg) => assert!(
            msg.contains("resolved outside projects_root"),
            "unexpected msg: {msg}"
        ),
        other => panic!("wrong error variant: {other:?}"),
    }
    // Must NOT have written through the symlink.
    assert!(!target.exists());
}
