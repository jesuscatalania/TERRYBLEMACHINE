//! Tauri IPC commands for the vectorizer (raster→SVG via VTracer).

use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use thiserror::Error;

use super::types::{VectorizeError, VectorizeInput, VectorizeResult, Vectorizer};

pub struct VectorizerState(pub Arc<dyn Vectorizer>);

impl VectorizerState {
    pub fn new(vectorizer: Arc<dyn Vectorizer>) -> Self {
        Self(vectorizer)
    }
}

#[derive(Debug, Serialize, Error)]
#[serde(tag = "kind", content = "detail")]
#[serde(rename_all = "kebab-case")]
pub enum VectorizeIpcError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("vtracer error: {0}")]
    Vtracer(String),
    #[error("io error: {0}")]
    Io(String),
}

impl From<VectorizeError> for VectorizeIpcError {
    fn from(value: VectorizeError) -> Self {
        match value {
            VectorizeError::InvalidInput(m) => Self::InvalidInput(m),
            VectorizeError::Vtracer(m) => Self::Vtracer(m),
            VectorizeError::Io(m) => Self::Io(m),
        }
    }
}

#[tauri::command]
pub async fn vectorize_image(
    input: VectorizeInput,
    state: State<'_, VectorizerState>,
) -> Result<VectorizeResult, VectorizeIpcError> {
    state.0.vectorize(input).await.map_err(Into::into)
}
