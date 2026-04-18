//! Tauri IPC commands for the mesh (3D) pipeline.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use thiserror::Error;

use super::types::{MeshImageInput, MeshPipeline, MeshPipelineError, MeshResult, MeshTextInput};
use crate::projects::commands::ProjectStoreState;

pub struct MeshPipelineState(pub Arc<dyn MeshPipeline>);

impl MeshPipelineState {
    pub fn new(pipeline: Arc<dyn MeshPipeline>) -> Self {
        Self(pipeline)
    }
}

#[derive(Debug, Serialize, Error)]
#[serde(tag = "kind", content = "detail")]
#[serde(rename_all = "kebab-case")]
pub enum MeshIpcError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("router error: {0}")]
    Router(String),
    #[error("no GLB output")]
    NoOutput,
    #[error("download failed: {0}")]
    Download(String),
    #[error("cache error: {0}")]
    Cache(String),
}

impl From<MeshPipelineError> for MeshIpcError {
    fn from(value: MeshPipelineError) -> Self {
        match value {
            MeshPipelineError::InvalidInput(m) => Self::InvalidInput(m),
            MeshPipelineError::Router(m) => Self::Router(m),
            MeshPipelineError::NoOutput => Self::NoOutput,
            MeshPipelineError::Download(m) => Self::Download(m),
            MeshPipelineError::Cache(m) => Self::Cache(m),
        }
    }
}

#[tauri::command]
pub async fn generate_mesh_from_text(
    input: MeshTextInput,
    state: State<'_, MeshPipelineState>,
) -> Result<MeshResult, MeshIpcError> {
    state.0.generate_from_text(input).await.map_err(Into::into)
}

#[tauri::command]
pub async fn generate_mesh_from_image(
    input: MeshImageInput,
    state: State<'_, MeshPipelineState>,
) -> Result<MeshResult, MeshIpcError> {
    state.0.generate_from_image(input).await.map_err(Into::into)
}

/// Copy a cached GLB at `local_path` to the user-chosen `target_path`.
///
/// Pass-through of the mesh cache: mesh generation (T8–T12) stores the GLB at
/// `~/Library/Caches/terryblemachine/meshes/<sha256>.glb`. "Export GLB" copies
/// that file to wherever the user asked (typically
/// `<project>/exports/<timestamp>-<suffix>-mesh.glb`), creating the parent
/// directory tree as needed. Missing source → `InvalidInput`; any IO failure
/// during mkdir/copy → `Cache`. Returns the absolute `target_path` on success
/// so the frontend can surface it in the success toast.
///
/// Trust boundary: `target_path` is caller-supplied (IPC) and MUST lie under
/// `projects_root`. See [`export_mesh_inner`] for the full guard logic and
/// [`crate::website_analyzer::commands::resolve_assets_dir`] for the pattern
/// this mirrors (FU #101).
#[tauri::command]
pub fn export_mesh(
    project_state: State<'_, ProjectStoreState>,
    local_path: PathBuf,
    target_path: PathBuf,
) -> Result<PathBuf, MeshIpcError> {
    export_mesh_inner(&project_state.root, &local_path, &target_path)
}

/// Inner logic for [`export_mesh`], extracted so unit tests can drive it
/// without a Tauri `State` wrapper.
///
/// Guard order mirrors `resolve_assets_dir`:
/// 1. Source must exist (cheap, early).
/// 2. Lexical `starts_with(projects_root)` check — cheap, catches `/etc/foo`
///    before we mkdir anything.
/// 3. `create_dir_all` on target's parent (idempotent).
/// 4. Canonicalise root + target parent and re-check `starts_with` — catches
///    symlink escapes the lexical check can't see.
/// 5. `copy` source → target.
pub fn export_mesh_inner(
    projects_root: &Path,
    local_path: &Path,
    target_path: &Path,
) -> Result<PathBuf, MeshIpcError> {
    if !local_path.exists() {
        return Err(MeshIpcError::InvalidInput(format!(
            "source mesh not in cache: {}",
            local_path.display()
        )));
    }

    // 1. Lexical prefix check — guards mkdir against paths like `/etc/foo`.
    if !target_path.starts_with(projects_root) {
        return Err(MeshIpcError::InvalidInput(format!(
            "target_path must be under projects_root ({}): {}",
            projects_root.display(),
            target_path.display()
        )));
    }

    // 2. mkdir -p the parent so canonicalize can resolve it.
    let parent = target_path
        .parent()
        .ok_or_else(|| MeshIpcError::InvalidInput("target_path has no parent".into()))?;
    std::fs::create_dir_all(parent)
        .map_err(|e| MeshIpcError::Cache(format!("mkdir target parent: {e}")))?;

    // 3. Canonical re-check defends against symlinks.
    let root_canon = std::fs::canonicalize(projects_root)
        .map_err(|e| MeshIpcError::Cache(format!("canon root: {e}")))?;
    let parent_canon = std::fs::canonicalize(parent)
        .map_err(|e| MeshIpcError::Cache(format!("canon target parent: {e}")))?;
    if !parent_canon.starts_with(&root_canon) {
        return Err(MeshIpcError::InvalidInput(format!(
            "target_path resolved outside projects_root: {}",
            parent_canon.display()
        )));
    }

    std::fs::copy(local_path, target_path)
        .map_err(|e| MeshIpcError::Cache(format!("copy failed: {e}")))?;
    Ok(target_path.to_path_buf())
}
