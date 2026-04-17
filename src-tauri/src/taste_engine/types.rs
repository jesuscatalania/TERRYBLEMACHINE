//! Shared data types for the taste engine.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A module-scoped extension to the global rules, e.g. "Für Websites → dark
/// mode first". Context tag is normalized to lowercase ascii so matching
/// against the current module is case-insensitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextRule {
    pub context: String,
    pub rules: Vec<String>,
}

/// Structured view of every `.md` file under `meingeschmack/regeln/`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TasteRules {
    pub preferred: Vec<String>,
    pub forbidden: Vec<String>,
    pub context_rules: Vec<ContextRule>,
    /// Named palettes; each key is the `## Palette` subheading (e.g. "Primär").
    #[serde(default)]
    pub palettes: Vec<Palette>,
    /// Hex colour values parsed out of any section.
    pub hex_colors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Palette {
    pub name: String,
    pub hex: Vec<String>,
}

/// One reference image, post-Claude-Vision analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageAnalysis {
    pub path: PathBuf,
    pub dominant_colors: Vec<String>,
    pub mood: Vec<String>,
    pub style_tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition: Option<String>,
    #[serde(default)]
    pub textures: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lighting: Option<String>,
}

/// The live profile served to the rest of the app.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StyleProfile {
    pub rules: TasteRules,
    pub analyses: Vec<ImageAnalysis>,
}
