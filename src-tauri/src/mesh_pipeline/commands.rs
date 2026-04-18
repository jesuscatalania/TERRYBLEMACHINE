//! Tauri IPC commands for the mesh (3D) pipeline.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use thiserror::Error;

use super::types::{MeshImageInput, MeshPipeline, MeshPipelineError, MeshResult, MeshTextInput};

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
