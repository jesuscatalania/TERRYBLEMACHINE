//! Tauri IPC commands for the brand-kit assembly pipeline.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use thiserror::Error;

use super::types::{BrandKitBuilder, BrandKitError, BrandKitInput, BrandKitResult};

pub struct BrandKitState(pub Arc<dyn BrandKitBuilder>);

impl BrandKitState {
    pub fn new(builder: Arc<dyn BrandKitBuilder>) -> Self {
        Self(builder)
    }
}

#[derive(Debug, Serialize, Error)]
#[serde(tag = "kind", content = "detail")]
#[serde(rename_all = "kebab-case")]
pub enum BrandKitIpcError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("image error: {0}")]
    Image(String),
    #[error("io error: {0}")]
    Io(String),
}

impl From<BrandKitError> for BrandKitIpcError {
    fn from(value: BrandKitError) -> Self {
        match value {
            BrandKitError::InvalidInput(m) => Self::InvalidInput(m),
            BrandKitError::Image(m) => Self::Image(m),
            BrandKitError::Io(m) => Self::Io(m),
        }
    }
}

#[tauri::command]
pub async fn build_brand_kit(
    input: BrandKitInput,
    state: State<'_, BrandKitState>,
) -> Result<BrandKitResult, BrandKitIpcError> {
    state.0.build(input).await.map_err(Into::into)
}
