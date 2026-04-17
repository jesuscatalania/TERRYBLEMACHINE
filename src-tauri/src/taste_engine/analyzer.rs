//! Vision-analysis interface. The real Claude-backed implementation calls
//! [`ClaudeClient`](crate::api_clients::claude::ClaudeClient) with a Vision
//! prompt; tests use [`StubVisionAnalyzer`] to return deterministic results.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;
use base64::prelude::{Engine as _, BASE64_STANDARD};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use super::errors::TasteError;
use super::types::ImageAnalysis;
use crate::ai_router::{AiRequest, AiRouter, Complexity, Priority, TaskKind};

#[async_trait]
pub trait VisionAnalyzer: Send + Sync {
    async fn analyze(&self, image: &Path) -> Result<ImageAnalysis, TasteError>;
}

// ─── Stub (used by tests / empty installations) ───────────────────────────

/// Test / development double. Returns whatever was pre-seeded for a path, or
/// a benign default.
#[derive(Default)]
pub struct StubVisionAnalyzer {
    seeded: Mutex<HashMap<PathBuf, ImageAnalysis>>,
}

impl StubVisionAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&self, path: PathBuf, analysis: ImageAnalysis) {
        self.seeded
            .lock()
            .expect("seeded mutex poisoned")
            .insert(path, analysis);
    }
}

#[async_trait]
impl VisionAnalyzer for StubVisionAnalyzer {
    async fn analyze(&self, image: &Path) -> Result<ImageAnalysis, TasteError> {
        let seeded = self.seeded.lock().expect("seeded mutex poisoned");
        if let Some(analysis) = seeded.get(image) {
            return Ok(analysis.clone());
        }
        Ok(ImageAnalysis {
            path: image.to_path_buf(),
            dominant_colors: Vec::new(),
            mood: Vec::new(),
            style_tags: Vec::new(),
            composition: None,
            textures: Vec::new(),
            lighting: None,
        })
    }
}

// ─── Claude Vision ───────────────────────────────────────────────────────

/// Strict JSON shape the prompt asks Claude to emit.
#[derive(Debug, Deserialize)]
struct VisionJson {
    #[serde(default)]
    palette: Vec<String>,
    #[serde(default)]
    style: Vec<String>,
}

const VISION_PROMPT: &str = "Extract palette (up to 6 hex colors) and 3–6 style keywords. Respond strictly as JSON: {\"palette\": [\"#rrggbb\"], \"style\": [\"…\"]}.";

/// Production analyzer backed by Claude's Vision Messages endpoint.
///
/// Routes image-analysis requests through the shared [`AiRouter`], which
/// handles provider selection (Claude Sonnet with Haiku fallback),
/// retries, caching, and budget accounting.
pub struct ClaudeVisionAnalyzer {
    router: Arc<AiRouter>,
}

impl ClaudeVisionAnalyzer {
    pub fn new(router: Arc<AiRouter>) -> Self {
        Self { router }
    }
}

fn mime_for(p: &Path) -> &'static str {
    match p
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "image/png",
    }
}

/// Pull the response text out of the wrapped `AiResponse.output`. The
/// Claude client wraps the raw Messages API response as
/// `{ "text": "...", "stop_reason": "..." }` (see
/// [`crate::api_clients::claude::ClaudeClient::send_messages`]).
fn response_text(output: &serde_json::Value) -> Option<String> {
    // Preferred: wrapped form.
    if let Some(s) = output.get("text").and_then(|v| v.as_str()) {
        return Some(s.to_string());
    }
    // Fallback: raw Messages API response `{ content: [{text: "..."}], ... }`.
    output
        .get("content")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|b| b.get("text"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

static HEX_REGEX: OnceLock<regex::Regex> = OnceLock::new();

/// Compiled-once hex matcher. The longest-form alternative is listed first so
/// the regex engine prefers an `#rrggbbaa` match over a partial `#rrggbb`
/// when both are valid at a given position.
fn hex_regex() -> &'static regex::Regex {
    HEX_REGEX.get_or_init(|| {
        // 8-char (rgba) first, then 6-char rgb, then 3-char shorthand.
        regex::Regex::new(r"#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{3})\b")
            .expect("valid regex")
    })
}

/// Best-effort hex extraction for when Claude deviates from the JSON-only
/// contract. Matches `#rrggbbaa`, `#rrggbb`, and `#rgb` literal colours.
fn extract_hex_colors(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for m in hex_regex().find_iter(text) {
        let hex = m.as_str().to_ascii_lowercase();
        if !out.contains(&hex) {
            out.push(hex);
        }
    }
    out
}

/// Stable SHA-256-derived id for a vision request.
///
/// Routing the request through the AiRouter populates the SemanticCache
/// keyed off `(prompt, payload)`, so two analyzer calls on the same image
/// path + mime + prompt now hit the cache instead of issuing duplicate
/// Claude Vision calls.
fn deterministic_id(path: &Path, mime: &str, prompt: &str) -> String {
    let mut h = Sha256::new();
    h.update(path.to_string_lossy().as_bytes());
    h.update(mime.as_bytes());
    h.update(prompt.as_bytes());
    format!("vision-{:x}", h.finalize())
}

#[async_trait]
impl VisionAnalyzer for ClaudeVisionAnalyzer {
    async fn analyze(&self, image: &Path) -> Result<ImageAnalysis, TasteError> {
        let bytes = std::fs::read(image)?;
        let b64 = BASE64_STANDARD.encode(&bytes);
        let mime = mime_for(image);

        let prompt = VISION_PROMPT.to_string();
        let req = AiRequest {
            id: deterministic_id(image, mime, &prompt),
            task: TaskKind::ImageAnalysis,
            priority: Priority::Normal,
            complexity: Complexity::Simple,
            prompt,
            payload: json!({
                "images": [{ "media_type": mime, "data": b64 }]
            }),
        };

        let resp = self
            .router
            .route(req)
            .await
            .map_err(|e| TasteError::Analysis(format!("router: {e}")))?;

        let text = response_text(&resp.output)
            .ok_or_else(|| TasteError::Analysis("claude vision: empty response".into()))?;

        // Prefer strict JSON. On failure, fall back to hex extraction so we
        // still surface *something* useful to the user.
        let (palette, style) = match serde_json::from_str::<VisionJson>(text.trim()) {
            Ok(v) => (v.palette, v.style),
            Err(_) => (extract_hex_colors(&text), Vec::new()),
        };

        Ok(ImageAnalysis {
            path: image.to_path_buf(),
            dominant_colors: palette,
            mood: Vec::new(),
            style_tags: style,
            composition: None,
            textures: Vec::new(),
            lighting: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ai_router::{
        AiClient, DefaultRoutingStrategy, PriorityQueue, Provider, RetryPolicy,
    };
    use crate::api_clients::claude::{ClaudeClient, KEYCHAIN_SERVICE};
    use crate::keychain::{InMemoryStore, KeyStore};
    use std::collections::HashMap;
    use std::io::Write;
    use std::time::Duration;

    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn stub_returns_default_for_unseeded_paths() {
        let a = StubVisionAnalyzer::new();
        let result = a.analyze(Path::new("/tmp/nope.png")).await.unwrap();
        assert_eq!(result.path, PathBuf::from("/tmp/nope.png"));
        assert!(result.dominant_colors.is_empty());
    }

    #[tokio::test]
    async fn stub_returns_seeded_data_when_available() {
        let a = StubVisionAnalyzer::new();
        let fake = ImageAnalysis {
            path: PathBuf::from("/tmp/a.png"),
            dominant_colors: vec!["#D4A373".into()],
            mood: vec!["warm".into()],
            style_tags: vec!["minimalist".into()],
            composition: Some("centred".into()),
            textures: vec!["matte".into()],
            lighting: Some("soft".into()),
        };
        a.seed(PathBuf::from("/tmp/a.png"), fake.clone());
        let got = a.analyze(Path::new("/tmp/a.png")).await.unwrap();
        assert_eq!(got, fake);
    }

    #[test]
    fn mime_for_known_extensions() {
        assert_eq!(mime_for(Path::new("a.png")), "image/png");
        assert_eq!(mime_for(Path::new("a.JPG")), "image/jpeg");
        assert_eq!(mime_for(Path::new("a.jpeg")), "image/jpeg");
        assert_eq!(mime_for(Path::new("a.webp")), "image/webp");
        assert_eq!(mime_for(Path::new("a.gif")), "image/gif");
        assert_eq!(mime_for(Path::new("a.unknown")), "image/png");
    }

    #[test]
    fn hex_extraction_picks_unique_colors() {
        let colors = extract_hex_colors("use #abcdef and also #ABCDEF, then #123");
        // Case-normalized + deduped; short form preserved.
        assert!(colors.contains(&"#abcdef".to_string()));
        assert!(colors.contains(&"#123".to_string()));
        assert_eq!(colors.len(), 2);
    }

    #[test]
    fn extract_hex_accepts_rgba_eight_char() {
        // 8-char hex (rgba) must be matched whole — not truncated to a 6-char
        // prefix. Mixing all three lengths must not interleave matches.
        let colors = extract_hex_colors("alpha #112233aa, opaque #445566, brief #abc");
        assert!(
            colors.contains(&"#112233aa".to_string()),
            "expected 8-char hex preserved, got {colors:?}"
        );
        assert!(colors.contains(&"#445566".to_string()));
        assert!(colors.contains(&"#abc".to_string()));
        assert_eq!(colors.len(), 3, "no duplicates expected, got {colors:?}");
    }

    #[test]
    fn deterministic_id_is_stable_across_calls() {
        let p = Path::new("/tmp/some/ref.png");
        let a = deterministic_id(p, "image/png", "prompt-A");
        let b = deterministic_id(p, "image/png", "prompt-A");
        assert_eq!(a, b);
        assert!(a.starts_with("vision-"));
        // Different inputs must yield distinct ids.
        assert_ne!(a, deterministic_id(p, "image/jpeg", "prompt-A"));
        assert_ne!(a, deterministic_id(p, "image/png", "prompt-B"));
        assert_ne!(
            a,
            deterministic_id(Path::new("/tmp/other/ref.png"), "image/png", "prompt-A")
        );
    }

    fn test_router(claude_uri: String) -> Arc<AiRouter> {
        let store = InMemoryStore::new();
        store.store(KEYCHAIN_SERVICE, "sk-test").unwrap();
        let key_store: Arc<dyn KeyStore> = Arc::new(store);
        let claude = Arc::new(ClaudeClient::for_test(key_store, claude_uri));
        let mut clients: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();
        clients.insert(Provider::Claude, claude);
        let retry = RetryPolicy {
            max_attempts: 1,
            base: Duration::from_millis(0),
            factor: 1,
            max: Duration::from_millis(0),
        };
        Arc::new(AiRouter::new(
            Arc::new(DefaultRoutingStrategy),
            clients,
            retry,
            Arc::new(PriorityQueue::new()),
        ))
    }

    #[tokio::test]
    async fn claude_vision_analyzer_extracts_palette_and_style() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": "{\"palette\":[\"#abcdef\"],\"style\":[\"warm\"]}"
                }],
                "stop_reason": "end_turn"
            })))
            .expect(1)
            .mount(&server)
            .await;

        // Real temp image file — analyzer reads it before dispatching.
        let tmp_dir = tempfile::tempdir().unwrap();
        let img_path = tmp_dir.path().join("ref.png");
        std::fs::File::create(&img_path)
            .unwrap()
            .write_all(&[0x89, 0x50, 0x4e, 0x47])
            .unwrap();

        let router = test_router(server.uri());
        let analyzer = ClaudeVisionAnalyzer::new(router);
        let got = analyzer.analyze(&img_path).await.unwrap();

        assert_eq!(got.path, img_path);
        assert_eq!(got.dominant_colors, vec!["#abcdef".to_string()]);
        assert_eq!(got.style_tags, vec!["warm".to_string()]);
    }

    #[tokio::test]
    async fn claude_vision_analyzer_falls_back_to_hex_on_bad_json() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": "not JSON but includes #112233 and #abcdef"
                }],
                "stop_reason": "end_turn"
            })))
            .mount(&server)
            .await;

        let tmp_dir = tempfile::tempdir().unwrap();
        let img_path = tmp_dir.path().join("ref.jpg");
        std::fs::File::create(&img_path)
            .unwrap()
            .write_all(&[0xff, 0xd8, 0xff])
            .unwrap();

        let router = test_router(server.uri());
        let analyzer = ClaudeVisionAnalyzer::new(router);
        let got = analyzer.analyze(&img_path).await.unwrap();

        assert!(got.dominant_colors.contains(&"#112233".to_string()));
        assert!(got.dominant_colors.contains(&"#abcdef".to_string()));
        assert!(got.style_tags.is_empty());
    }

    #[tokio::test]
    async fn claude_vision_analyzer_errors_on_missing_file() {
        let server = MockServer::start().await;
        let router = test_router(server.uri());
        let analyzer = ClaudeVisionAnalyzer::new(router);
        let err = analyzer
            .analyze(Path::new("/tmp/definitely-does-not-exist-xyz.png"))
            .await
            .unwrap_err();
        assert!(matches!(err, TasteError::Io(_)));
    }

    /// Twin of `claude_vision_analyzer_extracts_palette_and_style` covering
    /// the JPEG path. Asserts the outgoing Messages body sets the nested
    /// `media_type` to `image/jpeg`, which is the integration contract that
    /// `mime_for(".jpg")` is supposed to enforce end-to-end.
    #[tokio::test]
    async fn claude_vision_analyzer_handles_jpeg_media_type() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(body_partial_json(serde_json::json!({
                "messages": [{
                    "role": "user",
                    "content": [
                        { "type": "text" },
                        { "type": "image", "source": { "media_type": "image/jpeg" } }
                    ]
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": "{\"palette\":[\"#aabbcc\"],\"style\":[\"warm\"]}"
                }],
                "stop_reason": "end_turn"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let tmp_dir = tempfile::tempdir().unwrap();
        let img_path = tmp_dir.path().join("ref.jpg");
        std::fs::File::create(&img_path)
            .unwrap()
            .write_all(&[0xff, 0xd8, 0xff, 0xe0])
            .unwrap();

        let router = test_router(server.uri());
        let analyzer = ClaudeVisionAnalyzer::new(router);
        let got = analyzer.analyze(&img_path).await.unwrap();

        assert_eq!(got.dominant_colors, vec!["#aabbcc".to_string()]);
        assert_eq!(got.style_tags, vec!["warm".to_string()]);
    }

    /// Captured AiClient that records every request id passed through it.
    /// Used to prove the analyzer derives a deterministic id from
    /// (path, mime, prompt) instead of a fresh UUID each call.
    struct CapturingClient {
        calls: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl AiClient for CapturingClient {
        fn provider(&self) -> Provider {
            Provider::Claude
        }
        fn supports(&self, _model: crate::ai_router::Model) -> bool {
            true
        }
        async fn execute(
            &self,
            model: crate::ai_router::Model,
            request: &AiRequest,
        ) -> Result<crate::ai_router::AiResponse, crate::ai_router::ProviderError> {
            self.calls
                .lock()
                .expect("calls mutex poisoned")
                .push(request.id.clone());
            Ok(crate::ai_router::AiResponse {
                request_id: request.id.clone(),
                model,
                output: serde_json::json!({
                    "text": "{\"palette\":[\"#abcdef\"],\"style\":[]}"
                }),
                cost_cents: None,
                cached: false,
            })
        }
        async fn health_check(&self) -> bool {
            true
        }
        async fn get_usage(
            &self,
        ) -> Result<crate::ai_router::ProviderUsage, crate::ai_router::ProviderError> {
            Ok(crate::ai_router::ProviderUsage::default())
        }
    }

    #[tokio::test]
    async fn analyze_uses_deterministic_request_id() {
        let captured = Arc::new(CapturingClient {
            calls: Mutex::new(Vec::new()),
        });
        let mut clients: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();
        clients.insert(Provider::Claude, captured.clone() as Arc<dyn AiClient>);
        let retry = RetryPolicy {
            max_attempts: 1,
            base: Duration::from_millis(0),
            factor: 1,
            max: Duration::from_millis(0),
        };
        let router = Arc::new(AiRouter::new(
            Arc::new(DefaultRoutingStrategy),
            clients,
            retry,
            Arc::new(PriorityQueue::new()),
        ));

        let tmp_dir = tempfile::tempdir().unwrap();
        let img_path = tmp_dir.path().join("ref.png");
        std::fs::File::create(&img_path)
            .unwrap()
            .write_all(&[0x89, 0x50, 0x4e, 0x47])
            .unwrap();

        let analyzer = ClaudeVisionAnalyzer::new(router);
        let first = analyzer.analyze(&img_path).await.unwrap();
        let second = analyzer.analyze(&img_path).await.unwrap();

        // The first call must reach the AiClient with the deterministic id.
        // Second call may be served by the SemanticCache (which is itself a
        // function of (prompt, model, payload) — so a UUID-based id would
        // *also* hit the cache here). The discriminating evidence is the
        // exact id observed by the client on the first call: it must match
        // an independent recomputation, and the first/second analysis
        // results must be identical because both derive from the same
        // cached (or first-shot) AiResponse.
        let calls = captured.calls.lock().unwrap();
        assert!(
            !calls.is_empty(),
            "analyzer should issue at least one router call"
        );
        let expected = deterministic_id(&img_path, "image/png", VISION_PROMPT);
        for (i, observed) in calls.iter().enumerate() {
            assert_eq!(
                observed, &expected,
                "call #{i} request.id must match deterministic_id; got {observed}"
            );
        }
        assert_eq!(
            first.dominant_colors, second.dominant_colors,
            "deterministic id keeps cache stable across repeated analyses"
        );
    }
}
