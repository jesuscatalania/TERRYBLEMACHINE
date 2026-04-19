//! Production [`LogoPipeline`] that dispatches through [`AiRouter`] and
//! downloads every Ideogram variant into the platform cache directory.
//!
//! Mirrors [`crate::mesh_pipeline::pipeline`]: variants are requested in
//! parallel via [`futures::future::join_all`] with seed-salted payloads so
//! each variant hits a distinct cache key, then each remote URL is
//! SHA-256-hashed and stored at
//! `<cache-dir>/terryblemachine/logos/<sha256>.png`. The frontend prefers
//! the local path (piped through `convertFileSrc`) and falls back to the
//! remote URL when the download failed.
//!
//! Partial failure policy: if a single variant fails (provider error,
//! empty URL, download IO), we log to stderr and skip it. The call only
//! surfaces an error when every requested variant fails — the frontend
//! can still paint `N - k` logos rather than hard-failing on a flaky
//! network.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use futures::future::join_all;
use reqwest::Client;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::ai_router::{
    AiRequest, AiResponse, AiRouter, Complexity, Priority, RouterError, TaskKind,
};

use super::types::{LogoInput, LogoPipeline, LogoPipelineError, LogoVariant};

pub struct RouterLogoPipeline {
    router: Arc<AiRouter>,
    http: Client,
}

impl RouterLogoPipeline {
    pub fn new(router: Arc<AiRouter>) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("reqwest client builds with default TLS");
        Self { router, http }
    }

    fn cache_dir() -> Result<PathBuf, LogoPipelineError> {
        let base = dirs::cache_dir()
            .ok_or_else(|| LogoPipelineError::Cache("no platform cache dir".into()))?;
        let dir = base.join("terryblemachine").join("logos");
        std::fs::create_dir_all(&dir).map_err(|e| LogoPipelineError::Cache(e.to_string()))?;
        Ok(dir)
    }

    fn cache_path(remote_url: &str) -> Result<PathBuf, LogoPipelineError> {
        let mut h = Sha256::new();
        h.update(remote_url.as_bytes());
        let hash = format!("{:x}", h.finalize());
        let dir = Self::cache_dir()?;
        Ok(dir.join(format!("{hash}.png")))
    }

    /// Download `remote_url` into the cache, returning the local path. A
    /// cache hit (file already exists) short-circuits without re-fetching —
    /// Ideogram URLs are content-addressed per generation, so identical
    /// URLs are safe to cache indefinitely.
    ///
    /// `file://` URLs are special-cased so integration tests can exercise
    /// the full pipeline without wiring a mock HTTP server.
    async fn download_to_cache(&self, remote_url: &str) -> Result<PathBuf, LogoPipelineError> {
        let path = Self::cache_path(remote_url)?;
        if path.exists() {
            return Ok(path);
        }

        if let Some(stripped) = remote_url.strip_prefix("file://") {
            let src = Path::new(stripped);
            tokio::fs::copy(src, &path)
                .await
                .map_err(|e| LogoPipelineError::Download(e.to_string()))?;
            return Ok(path);
        }

        let bytes = self
            .http
            .get(remote_url)
            .send()
            .await
            .map_err(|e| LogoPipelineError::Download(e.to_string()))?
            .bytes()
            .await
            .map_err(|e| LogoPipelineError::Download(e.to_string()))?;
        tokio::fs::write(&path, &bytes)
            .await
            .map_err(|e| LogoPipelineError::Cache(e.to_string()))?;
        Ok(path)
    }
}

fn router_to_pipeline_err(err: RouterError) -> LogoPipelineError {
    LogoPipelineError::Router(err.to_string())
}

/// Pull a PNG URL out of an Ideogram response. The current Ideogram v3
/// client (`api_clients/ideogram.rs`) flattens the first element of the
/// upstream `data[]` array to `output.url`; we accept a few shapes
/// defensively so a future provider swap (e.g. a Replicate fallback) that
/// echoes a different envelope still parses.
fn extract_logo_url(resp: &AiResponse) -> Option<String> {
    resp.output
        .get("url")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .or_else(|| {
            resp.output
                .get("image")
                .and_then(|v| v.get("url"))
                .and_then(|v| v.as_str())
                .map(str::to_owned)
        })
        .or_else(|| {
            resp.output
                .get("data")
                .and_then(|a| a.get(0))
                .and_then(|e| e.get("url"))
                .and_then(|v| v.as_str())
                .map(str::to_owned)
        })
        .or_else(|| {
            resp.output
                .get("images")
                .and_then(|a| a.get(0))
                .and_then(|e| e.get("url"))
                .and_then(|v| v.as_str())
                .map(str::to_owned)
        })
}

/// Compose the per-variant prompt by joining the user prompt, the style
/// brief, and the optional palette hint with ". " separators.
fn build_variant_prompt(input: &LogoInput) -> String {
    let mut parts = vec![
        input.prompt.trim().to_string(),
        format!("Style: {}", input.style.brief()),
    ];
    if let Some(p) = &input.palette {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            parts.push(format!("Palette: {trimmed}"));
        }
    }
    parts.join(". ")
}

#[async_trait]
impl LogoPipeline for RouterLogoPipeline {
    async fn generate_variants(
        &self,
        input: LogoInput,
    ) -> Result<Vec<LogoVariant>, LogoPipelineError> {
        if input.prompt.trim().is_empty() {
            return Err(LogoPipelineError::InvalidInput("prompt is empty".into()));
        }
        let count = input.count.clamp(1, 10);
        let prompt = build_variant_prompt(&input);
        // `Model` is `Copy`, so capturing once and reassigning per-iteration
        // avoids any Arc/Clone dance while keeping every spawned request
        // independent.
        let override_capture = input.model_override;

        // Build the futures eagerly so join_all drives them concurrently.
        let mut futures = Vec::with_capacity(count as usize);
        for seed in 0..count {
            let router = Arc::clone(&self.router);
            let prompt = prompt.clone();
            let override_clone = override_capture;
            futures.push(async move {
                let req = AiRequest {
                    id: uuid::Uuid::new_v4().to_string(),
                    task: TaskKind::Logo,
                    priority: Priority::Normal,
                    complexity: Complexity::Medium,
                    prompt,
                    payload: json!({ "seed": seed }),
                    model_override: override_clone,
                };
                let resp = router.route(req).await.map_err(router_to_pipeline_err)?;
                let url = extract_logo_url(&resp).ok_or(LogoPipelineError::NoOutput)?;
                if url.is_empty() {
                    return Err(LogoPipelineError::NoOutput);
                }
                Ok::<(String, AiResponse, u32), LogoPipelineError>((url, resp, seed))
            });
        }

        let routed = join_all(futures).await;

        let mut variants = Vec::with_capacity(count as usize);
        for result in routed {
            match result {
                Ok((url, resp, seed)) => {
                    let local_path = match self.download_to_cache(&url).await {
                        Ok(p) => Some(p),
                        Err(e) => {
                            eprintln!(
                                "[logo-pipeline] download failed for {url}, falling back to remote URL: {e}"
                            );
                            None
                        }
                    };
                    variants.push(LogoVariant {
                        url,
                        local_path,
                        seed: Some(seed),
                        model: format!("{:?}", resp.model),
                    });
                }
                Err(e) => {
                    eprintln!("[logo-pipeline] variant failed: {e}");
                }
            }
        }

        if variants.is_empty() {
            return Err(LogoPipelineError::Router(format!(
                "all {count} variants failed"
            )));
        }
        Ok(variants)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_router::{DefaultRoutingStrategy, PriorityQueue, RetryPolicy};
    use std::collections::HashMap;

    fn zero_client_pipeline() -> RouterLogoPipeline {
        let router = Arc::new(AiRouter::new(
            Arc::new(DefaultRoutingStrategy),
            HashMap::new(),
            RetryPolicy::default_policy(),
            Arc::new(PriorityQueue::new()),
        ));
        RouterLogoPipeline::new(router)
    }

    #[tokio::test]
    async fn rejects_empty_prompt() {
        let p = zero_client_pipeline();
        let err = p
            .generate_variants(LogoInput {
                prompt: "   ".into(),
                style: crate::logo_pipeline::LogoStyle::Minimalist,
                count: 3,
                palette: None,
                module: "typography".into(),
                model_override: None,
            })
            .await
            .expect_err("empty prompt must be rejected before routing");
        assert!(
            matches!(err, LogoPipelineError::InvalidInput(_)),
            "expected InvalidInput, got {err:?}"
        );
    }

    #[test]
    fn cache_path_is_deterministic_for_same_url() {
        let a = RouterLogoPipeline::cache_path("https://ideogram.ai/img/123.png").unwrap();
        let b = RouterLogoPipeline::cache_path("https://ideogram.ai/img/123.png").unwrap();
        let c = RouterLogoPipeline::cache_path("https://ideogram.ai/img/456.png").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.extension().map(|e| e == "png").unwrap_or(false));
    }

    #[test]
    fn build_variant_prompt_joins_parts() {
        let p = build_variant_prompt(&LogoInput {
            prompt: "  Acme Corp  ".into(),
            style: crate::logo_pipeline::LogoStyle::Wordmark,
            count: 1,
            palette: Some("  monochrome ".into()),
            module: "typography".into(),
            model_override: None,
        });
        assert!(p.starts_with("Acme Corp. Style: "));
        assert!(p.ends_with("Palette: monochrome"));
    }

    #[test]
    fn build_variant_prompt_skips_blank_palette() {
        let p = build_variant_prompt(&LogoInput {
            prompt: "brand".into(),
            style: crate::logo_pipeline::LogoStyle::Emblem,
            count: 1,
            palette: Some("   ".into()),
            module: "typography".into(),
            model_override: None,
        });
        assert!(!p.contains("Palette"));
    }

    #[test]
    fn extract_logo_url_handles_top_level_url() {
        let resp = AiResponse {
            request_id: "x".into(),
            model: crate::ai_router::Model::IdeogramV3,
            output: json!({ "url": "https://out/x.png" }),
            cost_cents: None,
            cached: false,
        };
        assert_eq!(
            extract_logo_url(&resp),
            Some("https://out/x.png".to_string())
        );
    }

    #[test]
    fn extract_logo_url_handles_data_array() {
        let resp = AiResponse {
            request_id: "x".into(),
            model: crate::ai_router::Model::IdeogramV3,
            output: json!({ "data": [{ "url": "https://out/y.png" }] }),
            cost_cents: None,
            cached: false,
        };
        assert_eq!(
            extract_logo_url(&resp),
            Some("https://out/y.png".to_string())
        );
    }
}
