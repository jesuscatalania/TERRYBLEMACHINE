//! Tauri IPC command for the website analyzer.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use super::types::{AnalysisResult, AnalyzerError, UrlAnalyzer};
use crate::projects::commands::ProjectStoreState;

pub struct WebsiteAnalyzerState(pub Arc<dyn UrlAnalyzer>);

impl WebsiteAnalyzerState {
    pub fn new(analyzer: Arc<dyn UrlAnalyzer>) -> Self {
        Self(analyzer)
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum AnalyzerIpcError {
    InvalidUrl(String),
    Spawn(String),
    Sidecar(String),
    ParseOutput(String),
    /// Caller-supplied input failed validation (e.g. `project_path` outside
    /// the trusted projects root). Surfaces a typed error rather than a
    /// generic Io for clearer frontend handling.
    InvalidRequest(String),
    Io(String),
}

impl From<AnalyzerError> for AnalyzerIpcError {
    fn from(value: AnalyzerError) -> Self {
        match value {
            AnalyzerError::InvalidUrl(m) => Self::InvalidUrl(m),
            AnalyzerError::Spawn(m) => Self::Spawn(m),
            AnalyzerError::Sidecar(m) => Self::Sidecar(m),
            AnalyzerError::ParseOutput(m) => Self::ParseOutput(m),
            AnalyzerError::Io(e) => Self::Io(e.to_string()),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalyzeUrlInput {
    pub url: String,
    /// Optional screenshot target path (must be writable by the process).
    #[serde(default)]
    pub screenshot_path: Option<PathBuf>,
    /// Optional project path. When provided, referenced assets
    /// (images/icons/fonts) are downloaded into `<project_path>/assets`.
    #[serde(default)]
    pub project_path: Option<PathBuf>,
}

/// Resolve `project_path` (untrusted, IPC-supplied) to its canonical form
/// and refuse to proceed unless it lies inside `projects_root`.
///
/// Why: a symlink at `<project_path>/assets → /etc` would let the sidecar
/// write image/font files outside the user's projects folder. Canonicalizing
/// the requested path *and* the trusted root, then enforcing
/// `requested.starts_with(root)`, closes that escape hatch.
///
/// Defense-in-depth: we check `starts_with(root)` lexically *before*
/// `create_dir_all` so a caller passing e.g. `/etc/foo` cannot trick us
/// into creating directories outside the projects root. We then mkdir -p
/// the (now-known-safe) path so the New-Project flow can call `analyze_url`
/// before the directory has physically been written to disk. Finally we
/// canonicalize + re-check to catch symlink-based escapes that the lexical
/// prefix check can't see.
pub fn resolve_assets_dir(
    project_path: &Path,
    projects_root: &Path,
) -> Result<PathBuf, AnalyzerIpcError> {
    // 1. Lexical prefix check — guards mkdir against paths like `/etc/foo`.
    //    (Does not catch symlink escapes; canonicalize below does.)
    if !project_path.starts_with(projects_root) {
        return Err(AnalyzerIpcError::InvalidRequest(format!(
            "project_path `{}` must be under projects_root `{}`",
            project_path.display(),
            projects_root.display(),
        )));
    }
    // 2. Idempotent mkdir -p so canonicalize succeeds when the frontend's
    //    "New Project" flow races analyze_url.
    std::fs::create_dir_all(project_path).map_err(|e| {
        AnalyzerIpcError::InvalidRequest(format!(
            "project_path `{}` mkdir: {e}",
            project_path.display()
        ))
    })?;
    // 3. Canonicalize both sides and enforce the gate a second time —
    //    catches symlinks that point outside the trusted root.
    let canon = std::fs::canonicalize(project_path).map_err(|e| {
        AnalyzerIpcError::InvalidRequest(format!("project_path `{}`: {e}", project_path.display()))
    })?;
    let root_canon = std::fs::canonicalize(projects_root).map_err(|e| {
        AnalyzerIpcError::InvalidRequest(format!(
            "projects_root `{}`: {e}",
            projects_root.display()
        ))
    })?;
    if !canon.starts_with(&root_canon) {
        return Err(AnalyzerIpcError::InvalidRequest(format!(
            "project_path `{}` must be under projects_root `{}`",
            canon.display(),
            root_canon.display(),
        )));
    }
    Ok(canon.join("assets"))
}

#[tauri::command]
pub async fn analyze_url(
    input: AnalyzeUrlInput,
    state: State<'_, WebsiteAnalyzerState>,
    project_state: State<'_, ProjectStoreState>,
) -> Result<AnalysisResult, AnalyzerIpcError> {
    let assets_dir = match &input.project_path {
        Some(p) => Some(resolve_assets_dir(p, &project_state.root)?),
        None => None,
    };
    state
        .0
        .analyze(
            &input.url,
            input.screenshot_path.as_deref(),
            assets_dir.as_deref(),
        )
        .await
        .map_err(Into::into)
}
