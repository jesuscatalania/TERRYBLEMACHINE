//! Tauri IPC commands for the logo pipeline.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use thiserror::Error;

use super::types::{LogoInput, LogoPipeline, LogoPipelineError, LogoVariant};

pub struct LogoPipelineState(pub Arc<dyn LogoPipeline>);

impl LogoPipelineState {
    pub fn new(pipeline: Arc<dyn LogoPipeline>) -> Self {
        Self(pipeline)
    }
}

#[derive(Debug, Serialize, Error)]
#[serde(tag = "kind", content = "detail")]
#[serde(rename_all = "kebab-case")]
pub enum LogoIpcError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("router error: {0}")]
    Router(String),
    #[error("no image URL in response")]
    NoOutput,
    #[error("download failed: {0}")]
    Download(String),
    #[error("cache error: {0}")]
    Cache(String),
}

impl From<LogoPipelineError> for LogoIpcError {
    fn from(value: LogoPipelineError) -> Self {
        match value {
            LogoPipelineError::InvalidInput(m) => Self::InvalidInput(m),
            LogoPipelineError::Router(m) => Self::Router(m),
            LogoPipelineError::NoOutput => Self::NoOutput,
            LogoPipelineError::Download(m) => Self::Download(m),
            LogoPipelineError::Cache(m) => Self::Cache(m),
        }
    }
}

#[tauri::command]
pub async fn generate_logo_variants(
    input: LogoInput,
    state: State<'_, LogoPipelineState>,
) -> Result<Vec<LogoVariant>, LogoIpcError> {
    state.0.generate_variants(input).await.map_err(Into::into)
}
