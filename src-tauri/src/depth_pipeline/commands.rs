//! Tauri IPC commands for the depth-map pipeline.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use thiserror::Error;

use super::types::{DepthInput, DepthPipeline, DepthPipelineError, DepthResult};

pub struct DepthPipelineState(pub Arc<dyn DepthPipeline>);

impl DepthPipelineState {
    pub fn new(pipeline: Arc<dyn DepthPipeline>) -> Self {
        Self(pipeline)
    }
}

#[derive(Debug, Serialize, Error)]
#[serde(tag = "kind", content = "detail")]
pub enum DepthIpcError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("router error: {0}")]
    Router(String),
    #[error("no output from provider")]
    NoOutput,
}

impl From<DepthPipelineError> for DepthIpcError {
    fn from(value: DepthPipelineError) -> Self {
        match value {
            DepthPipelineError::InvalidInput(m) => Self::InvalidInput(m),
            DepthPipelineError::Router(m) => Self::Router(m),
            DepthPipelineError::NoOutput => Self::NoOutput,
        }
    }
}

#[tauri::command]
pub async fn generate_depth(
    input: DepthInput,
    state: State<'_, DepthPipelineState>,
) -> Result<DepthResult, DepthIpcError> {
    state.0.generate(input).await.map_err(Into::into)
}
