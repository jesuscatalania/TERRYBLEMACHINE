//! Tauri IPC commands for the Shotstack video-assembly pipeline.
//!
//! Mirrors [`crate::mesh_pipeline::commands`]: a `VideoAssemblerState` handle
//! (wrapping `Arc<dyn VideoAssembler>`), a serializable IPC error with the
//! same `{ kind, detail }` kebab-case shape the rest of the frontend expects,
//! and a `#[tauri::command] assemble_video` entry point.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use thiserror::Error;

use super::types::{AssemblyError, AssemblyInput, AssemblyResult, VideoAssembler};

pub struct VideoAssemblerState(pub Arc<dyn VideoAssembler>);

impl VideoAssemblerState {
    pub fn new(assembler: Arc<dyn VideoAssembler>) -> Self {
        Self(assembler)
    }
}

#[derive(Debug, Serialize, Error)]
#[serde(tag = "kind", content = "detail")]
#[serde(rename_all = "kebab-case")]
pub enum AssemblyIpcError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("provider error: {0}")]
    Provider(String),
    #[error("download failed: {0}")]
    Download(String),
    #[error("cache error: {0}")]
    Cache(String),
}

impl From<AssemblyError> for AssemblyIpcError {
    fn from(value: AssemblyError) -> Self {
        match value {
            AssemblyError::InvalidInput(m) => Self::InvalidInput(m),
            AssemblyError::Provider(m) => Self::Provider(m),
            AssemblyError::Download(m) => Self::Download(m),
            AssemblyError::Cache(m) => Self::Cache(m),
        }
    }
}

#[tauri::command]
pub async fn assemble_video(
    state: State<'_, VideoAssemblerState>,
    input: AssemblyInput,
) -> Result<AssemblyResult, AssemblyIpcError> {
    state.0.assemble(input).await.map_err(Into::into)
}
