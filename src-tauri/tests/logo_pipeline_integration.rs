//! Integration tests for the logo pipeline.
//!
//! These drive `RouterLogoPipeline` against a stub Ideogram client that
//! echoes a `file://` URL the pipeline then copies into the platform cache.
//! The full round-trip (router → provider → download → local_path) is
//! exercised without any real network IO.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::json;
use tempfile::TempDir;

use terryblemachine_lib::ai_router::{
    AiClient, AiRequest, AiResponse, AiRouter, DefaultRoutingStrategy, Model, PriorityQueue,
    Provider, ProviderError, ProviderUsage, RetryPolicy,
};
use terryblemachine_lib::logo_pipeline::{LogoInput, LogoPipeline, LogoStyle, RouterLogoPipeline};

// ─── Test double ──────────────────────────────────────────────────────

/// Stub `AiClient` for Ideogram: captures every request and returns a
/// provider-shaped response whose `data[0].url` is computed by the
/// caller-supplied closure. This lets individual tests choose whether all
/// variants share a URL (idempotency tests) or each variant gets its own
/// (distinct-cache tests).
struct StubIdeogram {
    captured: Arc<Mutex<Vec<AiRequest>>>,
    url_provider: Arc<dyn Fn(&AiRequest) -> String + Send + Sync>,
}

impl StubIdeogram {
    fn new(url_provider: Arc<dyn Fn(&AiRequest) -> String + Send + Sync>) -> Self {
        Self {
            captured: Arc::new(Mutex::new(Vec::new())),
            url_provider,
        }
    }

    fn captured(&self) -> Vec<AiRequest> {
        self.captured.lock().expect("mutex").clone()
    }
}

#[async_trait]
impl AiClient for StubIdeogram {
    fn provider(&self) -> Provider {
        Provider::Ideogram
    }

    fn supports(&self, m: Model) -> bool {
        matches!(m, Model::IdeogramV3)
    }

    async fn execute(&self, _model: Model, req: &AiRequest) -> Result<AiResponse, ProviderError> {
        self.captured.lock().expect("mutex").push(req.clone());
        let url = (self.url_provider)(req);
        Ok(AiResponse {
            request_id: req.id.clone(),
            model: Model::IdeogramV3,
            output: json!({ "data": [{ "url": url }] }),
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

fn pipeline_with_file_scheme(fake_png: PathBuf) -> (RouterLogoPipeline, Arc<StubIdeogram>) {
    let url = format!("file://{}", fake_png.display());
    let stub = Arc::new(StubIdeogram::new(Arc::new(move |_| url.clone())));
    let mut clients: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();
    clients.insert(Provider::Ideogram, stub.clone() as Arc<dyn AiClient>);
    let router = Arc::new(AiRouter::new(
        Arc::new(DefaultRoutingStrategy),
        clients,
        RetryPolicy::default_policy(),
        Arc::new(PriorityQueue::new()),
    ));
    (RouterLogoPipeline::new(router), stub)
}

// ─── Tests ────────────────────────────────────────────────────────────

#[tokio::test]
async fn generates_variants_via_ideogram_route() {
    let tmp = TempDir::new().unwrap();
    let img = tmp.path().join("fake.png");
    std::fs::write(&img, b"PNG").unwrap();
    let (p, stub) = pipeline_with_file_scheme(img);

    let variants = p
        .generate_variants(LogoInput {
            prompt: "TERRYBLEMACHINE".into(),
            style: LogoStyle::Minimalist,
            count: 5,
            palette: None,
            module: "typography".into(),
            model_override: None,
        })
        .await
        .unwrap();

    assert_eq!(variants.len(), 5);
    for v in &variants {
        assert!(v.url.contains("fake.png"));
        assert_eq!(v.model, "IdeogramV3");
        assert!(v.local_path.is_some());
    }
    assert_eq!(stub.captured().len(), 5);
    // Every request must carry the Logo task + Medium complexity so the
    // router picks IdeogramV3 per Phase-2 strategy.
    for req in stub.captured() {
        assert!(matches!(
            req.task,
            terryblemachine_lib::ai_router::TaskKind::Logo
        ));
    }
}

#[tokio::test]
async fn generate_variants_rejects_empty_prompt() {
    let tmp = TempDir::new().unwrap();
    let img = tmp.path().join("fake.png");
    std::fs::write(&img, b"PNG").unwrap();
    let (p, _) = pipeline_with_file_scheme(img);

    let err = p
        .generate_variants(LogoInput {
            prompt: "  ".into(),
            style: LogoStyle::Minimalist,
            count: 3,
            palette: None,
            module: "typography".into(),
            model_override: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        terryblemachine_lib::logo_pipeline::LogoPipelineError::InvalidInput(_)
    ));
}

#[tokio::test]
async fn generate_variants_caps_count_at_10() {
    let tmp = TempDir::new().unwrap();
    let img = tmp.path().join("fake.png");
    std::fs::write(&img, b"PNG").unwrap();
    let (p, stub) = pipeline_with_file_scheme(img);

    let variants = p
        .generate_variants(LogoInput {
            prompt: "brand".into(),
            style: LogoStyle::Wordmark,
            count: 25,
            palette: Some("monochrome".into()),
            module: "typography".into(),
            model_override: None,
        })
        .await
        .unwrap();
    assert_eq!(variants.len(), 10);
    assert_eq!(stub.captured().len(), 10);
}

#[tokio::test]
async fn download_is_idempotent_across_calls() {
    let tmp = TempDir::new().unwrap();
    let img = tmp.path().join("idem.png");
    std::fs::write(&img, b"IDEM").unwrap();
    let (p, _) = pipeline_with_file_scheme(img);

    let v1 = p
        .generate_variants(LogoInput {
            prompt: "logo".into(),
            style: LogoStyle::Emblem,
            count: 1,
            palette: None,
            module: "typography".into(),
            model_override: None,
        })
        .await
        .unwrap();
    let v2 = p
        .generate_variants(LogoInput {
            prompt: "logo".into(),
            style: LogoStyle::Emblem,
            count: 1,
            palette: None,
            module: "typography".into(),
            model_override: None,
        })
        .await
        .unwrap();

    // Both map the same URL → same local_path in the cache dir.
    assert_eq!(v1[0].local_path, v2[0].local_path);
}
