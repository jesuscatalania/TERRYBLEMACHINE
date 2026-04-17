//! Tauri IPC commands for the image pipeline.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use super::types::{
    GenerateVariantsInput, Image2ImageInput, ImagePipeline, ImagePipelineError, ImageResult,
    Text2ImageInput, UpscaleInput,
};

pub struct ImagePipelineState(pub Arc<dyn ImagePipeline>);

impl ImagePipelineState {
    pub fn new(pipeline: Arc<dyn ImagePipeline>) -> Self {
        Self(pipeline)
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum ImagePipelineIpcError {
    InvalidInput(String),
    Router(String),
    EmptyResponse,
    AllVariantsFailed(u32),
}

impl From<ImagePipelineError> for ImagePipelineIpcError {
    fn from(value: ImagePipelineError) -> Self {
        match value {
            ImagePipelineError::InvalidInput(m) => Self::InvalidInput(m),
            ImagePipelineError::Router(m) => Self::Router(m),
            ImagePipelineError::EmptyResponse => Self::EmptyResponse,
            ImagePipelineError::AllVariantsFailed(n) => Self::AllVariantsFailed(n),
        }
    }
}

#[tauri::command]
pub async fn text_to_image(
    input: Text2ImageInput,
    state: State<'_, ImagePipelineState>,
) -> Result<ImageResult, ImagePipelineIpcError> {
    state.0.text_to_image(input).await.map_err(Into::into)
}

#[tauri::command]
pub async fn image_to_image(
    input: Image2ImageInput,
    state: State<'_, ImagePipelineState>,
) -> Result<ImageResult, ImagePipelineIpcError> {
    state.0.image_to_image(input).await.map_err(Into::into)
}

#[tauri::command]
pub async fn upscale_image(
    input: UpscaleInput,
    state: State<'_, ImagePipelineState>,
) -> Result<ImageResult, ImagePipelineIpcError> {
    state.0.upscale(input).await.map_err(Into::into)
}

#[tauri::command]
pub async fn generate_variants(
    input: GenerateVariantsInput,
    state: State<'_, ImagePipelineState>,
) -> Result<Vec<ImageResult>, ImagePipelineIpcError> {
    state.0.variants(input).await.map_err(Into::into)
}
