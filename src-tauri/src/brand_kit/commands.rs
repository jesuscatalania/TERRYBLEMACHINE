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

#[tauri::command]
pub async fn export_brand_kit(
    state: State<'_, BrandKitState>,
    input: BrandKitInput,
    destination: std::path::PathBuf,
) -> Result<std::path::PathBuf, BrandKitIpcError> {
    // Compute the brand slug BEFORE the `.await` that moves `input` into
    // `build` — cloning the whole input just to keep the brand name around
    // would be wasteful, and borrowing across the await isn't possible
    // because `build` consumes the value.
    let brand_slug = super::export::slug_for(&input.brand_name);
    let result = state
        .0
        .build(input)
        .await
        .map_err(Into::<BrandKitIpcError>::into)?;

    // `write_zip` does disk I/O + Deflate compression of ~11 PNGs (up to a
    // few MB each at 2048×2048) — running it synchronously from an async
    // command stalls the Tokio runtime. Mirror the `spawn_blocking` pattern
    // used by `vectorizer::pipeline` and `brand_kit::pipeline` so other IPC
    // calls stay responsive while the ZIP is built.
    let assets = result.assets;
    let path = tokio::task::spawn_blocking(move || {
        super::export::write_zip(&destination, &brand_slug, &assets)
    })
    .await
    .map_err(|e| BrandKitIpcError::Io(format!("join error: {e}")))??;
    Ok(path)
}
