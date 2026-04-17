//! Taste Engine — applies the user's personal `meingeschmack/` style rules
//! to every generative prompt.
//!
//! Pipeline (see `docs/MEINGESCHMACK-SYSTEM.md`):
//! 1. **watcher** notices changes in the `meingeschmack/` folder and
//!    triggers a refresh.
//! 2. **parser** reads Markdown rule files into structured [`TasteRules`].
//! 3. **analyzer** runs reference images through Claude Vision (via the
//!    [`VisionAnalyzer`] trait) to extract dominant colors / mood / style
//!    tags.
//! 4. **enricher** combines the parsed rules + analyses into an augmented
//!    prompt for downstream generation.
//! 5. **negative** derives the "avoid these" prompt string.
//!
//! For Schritt 2.5 the parser, enricher, and negative generator are fully
//! implemented and tested. The analyzer and watcher expose the right
//! interfaces with stubs / thin wrappers so richer behaviour can slot in
//! during Phase 3+ when the frontend actually ships prompts.

pub mod analyzer;
pub mod commands;
mod enricher;
mod errors;
mod negative;
mod parser;
mod types;
pub mod watcher;

pub use analyzer::{ClaudeVisionAnalyzer, StubVisionAnalyzer, VisionAnalyzer};
pub use enricher::{enrich_prompt, EnrichOptions};
pub use errors::TasteError;
pub use negative::build_negative_prompt;
pub use parser::{parse_markdown_rules, parse_meingeschmack_dir};
pub use types::{ContextRule, ImageAnalysis, StyleProfile, TasteRules};
pub use watcher::{TasteWatcher, WatchEvent};

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

/// Top-level engine: owns the live [`StyleProfile`] and the services needed
/// to refresh it.
pub struct TasteEngine {
    root: PathBuf,
    profile: RwLock<StyleProfile>,
    analyzer: Arc<dyn VisionAnalyzer>,
}

impl TasteEngine {
    pub fn new(root: PathBuf, analyzer: Arc<dyn VisionAnalyzer>) -> Self {
        Self {
            root,
            profile: RwLock::new(StyleProfile::default()),
            analyzer,
        }
    }

    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    /// Re-read `meingeschmack/` from disk and, optionally, re-run image
    /// analyses. Silent-success when the folder does not exist so tests
    /// can exercise empty installations.
    pub async fn refresh(&self) -> Result<StyleProfile, TasteError> {
        let rules = parse_meingeschmack_dir(&self.root)?;
        let analyses = self.analyze_reference_images().await?;
        let profile = StyleProfile { rules, analyses };
        let mut guard = self.profile.write().await;
        *guard = profile.clone();
        Ok(profile)
    }

    async fn analyze_reference_images(&self) -> Result<Vec<ImageAnalysis>, TasteError> {
        let images_dir = self.root.join("referenzen").join("bilder");
        if !images_dir.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&images_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            // Only run the analyzer on recognised image extensions.
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase());
            if !matches!(
                ext.as_deref(),
                Some("png" | "jpg" | "jpeg" | "webp" | "gif")
            ) {
                continue;
            }
            match self.analyzer.analyze(&path).await {
                Ok(analysis) => out.push(analysis),
                // Skip unreadable images rather than abort the whole refresh.
                Err(_) => continue,
            }
        }
        Ok(out)
    }

    pub async fn profile(&self) -> StyleProfile {
        self.profile.read().await.clone()
    }

    /// Enrich a user prompt with the live style profile.
    pub async fn enrich(&self, prompt: &str, opts: &EnrichOptions) -> String {
        let profile = self.profile.read().await;
        enrich_prompt(prompt, &profile.rules, opts)
    }

    /// Build the negative prompt from the live style profile.
    pub async fn negative_prompt(&self) -> String {
        let profile = self.profile.read().await;
        build_negative_prompt(&profile.rules)
    }
}
