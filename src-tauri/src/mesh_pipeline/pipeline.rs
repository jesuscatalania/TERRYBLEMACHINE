//! Production [`MeshPipeline`] that dispatches through [`AiRouter`] and
//! downloads the resulting GLB into the platform cache directory.
//!
//! The download step is the distinguishing feature vs. `image_pipeline` /
//! `depth_pipeline`: Meshy returns a remote GLB URL (resolved via T8/T9
//! polling) but Three.js running inside the Tauri webview needs a local
//! `asset://` URL to avoid CORS/HTTP timeouts on first paint. So we hash
//! the remote URL (sha256), store at `~/Library/Caches/terryblemachine/
//! meshes/<hash>.glb`, and return that path for the frontend to pipe
//! through `convertFileSrc`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::ai_router::{
    AiRequest, AiResponse, AiRouter, Complexity, Priority, RouterError, TaskKind,
};

use super::types::{MeshImageInput, MeshPipeline, MeshPipelineError, MeshResult, MeshTextInput};

pub struct RouterMeshPipeline {
    router: Arc<AiRouter>,
    http: Client,
}

impl RouterMeshPipeline {
    pub fn new(router: Arc<AiRouter>) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("reqwest client builds with default TLS");
        Self { router, http }
    }

    fn cache_dir() -> Result<PathBuf, MeshPipelineError> {
        let base = dirs::cache_dir()
            .ok_or_else(|| MeshPipelineError::Cache("no platform cache dir".into()))?;
        let dir = base.join("terryblemachine").join("meshes");
        std::fs::create_dir_all(&dir).map_err(|e| MeshPipelineError::Cache(e.to_string()))?;
        Ok(dir)
    }

    fn cache_path(remote_url: &str) -> Result<PathBuf, MeshPipelineError> {
        let mut h = Sha256::new();
        h.update(remote_url.as_bytes());
        let hash = format!("{:x}", h.finalize());
        let dir = Self::cache_dir()?;
        Ok(dir.join(format!("{hash}.glb")))
    }

    /// Download `remote_url` into the cache, returning the local path. A
    /// cache hit (file already exists) short-circuits without re-fetching —
    /// Meshy URLs are content-addressed via task ID, so identical URLs are
    /// safe to cache indefinitely.
    ///
    /// `file://` URLs are special-cased so integration tests can exercise
    /// the full pipeline without wiring a mock HTTP server.
    async fn download_to_cache(&self, remote_url: &str) -> Result<PathBuf, MeshPipelineError> {
        let path = Self::cache_path(remote_url)?;
        if path.exists() {
            return Ok(path);
        }

        if let Some(stripped) = remote_url.strip_prefix("file://") {
            let src = Path::new(stripped);
            tokio::fs::copy(src, &path)
                .await
                .map_err(|e| MeshPipelineError::Download(e.to_string()))?;
            return Ok(path);
        }

        let bytes = self
            .http
            .get(remote_url)
            .send()
            .await
            .map_err(|e| MeshPipelineError::Download(e.to_string()))?
            .bytes()
            .await
            .map_err(|e| MeshPipelineError::Download(e.to_string()))?;
        tokio::fs::write(&path, &bytes)
            .await
            .map_err(|e| MeshPipelineError::Cache(e.to_string()))?;
        Ok(path)
    }

    fn extract_glb_url(resp: &AiResponse) -> Option<String> {
        resp.output
            .get("glb_url")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    }
}

fn router_to_pipeline_err(err: RouterError) -> MeshPipelineError {
    MeshPipelineError::Router(err.to_string())
}

#[async_trait]
impl MeshPipeline for RouterMeshPipeline {
    async fn generate_from_text(
        &self,
        input: MeshTextInput,
    ) -> Result<MeshResult, MeshPipelineError> {
        if input.prompt.trim().is_empty() {
            return Err(MeshPipelineError::InvalidInput("prompt is empty".into()));
        }
        let req = AiRequest {
            id: uuid::Uuid::new_v4().to_string(),
            task: TaskKind::Text3D,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: input.prompt,
            payload: json!({}),
            model_override: None,
        };
        let resp = self
            .router
            .route(req)
            .await
            .map_err(router_to_pipeline_err)?;
        let glb_url = Self::extract_glb_url(&resp).ok_or(MeshPipelineError::NoOutput)?;
        // Best-effort download: a failed fetch falls back to `None` so the
        // frontend can still render from the remote URL. Log the failure so
        // silent flaky-network regressions surface (FU #146).
        let local_path = match self.download_to_cache(&glb_url).await {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!(
                    "[mesh-pipeline] download failed for {glb_url}, falling back to remote URL: {e}"
                );
                None
            }
        };
        Ok(MeshResult {
            glb_url,
            local_path,
            model: format!("{:?}", resp.model),
        })
    }

    async fn generate_from_image(
        &self,
        input: MeshImageInput,
    ) -> Result<MeshResult, MeshPipelineError> {
        if input.image_url.trim().is_empty() {
            return Err(MeshPipelineError::InvalidInput("image_url required".into()));
        }
        if input.image_url.starts_with("data:") {
            return Err(MeshPipelineError::InvalidInput(
                "mesh: hosted image URL required — data-URLs unsupported".into(),
            ));
        }
        // `quick_preview` opts into the TripoSR routing tier (cheaper + faster
        // but lower fidelity). Without it we stay Meshy-primary.
        let complexity = if input.quick_preview {
            Complexity::Simple
        } else {
            Complexity::Medium
        };
        let req = AiRequest {
            id: uuid::Uuid::new_v4().to_string(),
            task: TaskKind::Image3D,
            priority: Priority::Normal,
            complexity,
            prompt: input.prompt.unwrap_or_default(),
            payload: json!({ "image_url": input.image_url }),
            model_override: None,
        };
        let resp = self
            .router
            .route(req)
            .await
            .map_err(router_to_pipeline_err)?;
        let glb_url = Self::extract_glb_url(&resp).ok_or(MeshPipelineError::NoOutput)?;
        let local_path = match self.download_to_cache(&glb_url).await {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!(
                    "[mesh-pipeline] download failed for {glb_url}, falling back to remote URL: {e}"
                );
                None
            }
        };
        Ok(MeshResult {
            glb_url,
            local_path,
            model: format!("{:?}", resp.model),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::ai_router::{DefaultRoutingStrategy, PriorityQueue, RetryPolicy};

    fn zero_client_pipeline() -> RouterMeshPipeline {
        let router = Arc::new(AiRouter::new(
            Arc::new(DefaultRoutingStrategy),
            HashMap::new(),
            RetryPolicy::default_policy(),
            Arc::new(PriorityQueue::new()),
        ));
        RouterMeshPipeline::new(router)
    }

    #[tokio::test]
    async fn text_rejects_empty_prompt() {
        let p = zero_client_pipeline();
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
    async fn image_rejects_data_url() {
        let p = zero_client_pipeline();
        let err = p
            .generate_from_image(MeshImageInput {
                image_url: "data:image/png;base64,iVBORw0KGgo=".into(),
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
            "error message should mention data-URLs, got: {err}"
        );
    }

    #[test]
    fn cache_path_is_deterministic_for_same_url() {
        let a = RouterMeshPipeline::cache_path("https://fake/meshy/123.glb").unwrap();
        let b = RouterMeshPipeline::cache_path("https://fake/meshy/123.glb").unwrap();
        let c = RouterMeshPipeline::cache_path("https://fake/meshy/456.glb").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.extension().map(|e| e == "glb").unwrap_or(false));
    }
}
