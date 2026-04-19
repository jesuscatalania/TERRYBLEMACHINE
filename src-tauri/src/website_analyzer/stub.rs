//! In-process double used by tests and in hosts where Node isn't available.

use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;
use parking_lot::Mutex;

use super::types::{AnalysisResult, AnalyzerError, UrlAnalyzer};

#[derive(Default)]
pub struct StubUrlAnalyzer {
    seeded: Mutex<HashMap<String, AnalysisResult>>,
    /// When set, all calls return this error regardless of input.
    force_error: Mutex<Option<String>>,
}

impl StubUrlAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&self, url: impl Into<String>, result: AnalysisResult) {
        self.seeded.lock().insert(url.into(), result);
    }

    pub fn force_error(&self, message: impl Into<String>) {
        *self.force_error.lock() = Some(message.into());
    }
}

fn default_result(url: &str) -> AnalysisResult {
    AnalysisResult {
        url: url.to_string(),
        status: 200,
        title: String::new(),
        description: None,
        colors: Vec::new(),
        fonts: Vec::new(),
        spacing: Vec::new(),
        custom_properties: HashMap::new(),
        layout: "other".to_string(),
        screenshot_path: None,
        assets: Vec::new(),
    }
}

#[async_trait]
impl UrlAnalyzer for StubUrlAnalyzer {
    async fn analyze(
        &self,
        url: &str,
        _screenshot_path: Option<&Path>,
        _assets_dir: Option<&Path>,
    ) -> Result<AnalysisResult, AnalyzerError> {
        if let Some(msg) = self.force_error.lock().clone() {
            return Err(AnalyzerError::Sidecar(msg));
        }
        let seeded = self.seeded.lock();
        Ok(seeded
            .get(url)
            .cloned()
            .unwrap_or_else(|| default_result(url)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_default_for_unseeded_url() {
        let a = StubUrlAnalyzer::new();
        let r = a.analyze("https://example.com", None, None).await.unwrap();
        assert_eq!(r.url, "https://example.com");
        assert_eq!(r.status, 200);
        assert!(r.colors.is_empty());
    }

    #[tokio::test]
    async fn returns_seeded_result_when_set() {
        let a = StubUrlAnalyzer::new();
        let seeded = AnalysisResult {
            url: "https://foo".into(),
            status: 200,
            title: "Foo".into(),
            description: Some("A foo site".into()),
            colors: vec!["rgb(255, 0, 0)".into()],
            fonts: vec!["Inter".into()],
            spacing: vec!["8px".into()],
            custom_properties: HashMap::new(),
            layout: "grid".into(),
            screenshot_path: None,
            assets: Vec::new(),
        };
        a.seed("https://foo", seeded.clone());
        let r = a.analyze("https://foo", None, None).await.unwrap();
        assert_eq!(r, seeded);
    }

    #[tokio::test]
    async fn force_error_overrides_seeding() {
        let a = StubUrlAnalyzer::new();
        a.seed(
            "https://foo",
            AnalysisResult {
                url: "https://foo".into(),
                status: 200,
                title: "x".into(),
                description: None,
                colors: vec![],
                fonts: vec![],
                spacing: vec![],
                custom_properties: HashMap::new(),
                layout: "other".into(),
                screenshot_path: None,
                assets: Vec::new(),
            },
        );
        a.force_error("playwright crashed");
        let err = a.analyze("https://foo", None, None).await.unwrap_err();
        assert!(matches!(err, AnalyzerError::Sidecar(_)));
    }
}
