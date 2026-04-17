//! Vision-analysis interface. The real Claude-backed implementation calls
//! [`ClaudeClient`](crate::api_clients::claude::ClaudeClient) with a Vision
//! prompt; tests use [`StubVisionAnalyzer`] to return deterministic results.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::errors::TasteError;
use super::types::ImageAnalysis;
use crate::keychain::KeyStore;

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

// ─── Claude Vision (scaffold) ────────────────────────────────────────────

/// Production analyzer backed by Claude's Vision Messages endpoint.
///
/// Schritt 2.5 ships only the scaffold — the inner HTTP call lives in
/// `api_clients/claude.rs` and will be wired end-to-end when a module
/// actually needs image analysis (Phase 3+). For now `analyze()` returns
/// a descriptive `TasteError::Analysis` so callers can proceed with an
/// empty analysis list.
pub struct ClaudeVisionAnalyzer {
    #[allow(dead_code)]
    key_store: Arc<dyn KeyStore>,
}

impl ClaudeVisionAnalyzer {
    pub fn new(key_store: Arc<dyn KeyStore>) -> Self {
        Self { key_store }
    }
}

#[async_trait]
impl VisionAnalyzer for ClaudeVisionAnalyzer {
    async fn analyze(&self, _image: &Path) -> Result<ImageAnalysis, TasteError> {
        Err(TasteError::Analysis(
            "Claude Vision integration is scheduled for Phase 3+".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
