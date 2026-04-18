//! Tauri IPC commands for the video pipeline.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use thiserror::Error;

use super::types::{
    VideoImageInput, VideoPipeline, VideoPipelineError, VideoResult, VideoTextInput,
};

pub struct VideoPipelineState(pub Arc<dyn VideoPipeline>);

impl VideoPipelineState {
    pub fn new(pipeline: Arc<dyn VideoPipeline>) -> Self {
        Self(pipeline)
    }
}

#[derive(Debug, Serialize, Error)]
#[serde(tag = "kind", content = "detail")]
#[serde(rename_all = "kebab-case")]
pub enum VideoIpcError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("router error: {0}")]
    Router(String),
    #[error("no video output")]
    NoOutput,
    #[error("download failed: {0}")]
    Download(String),
    #[error("cache error: {0}")]
    Cache(String),
}

impl From<VideoPipelineError> for VideoIpcError {
    fn from(value: VideoPipelineError) -> Self {
        match value {
            VideoPipelineError::InvalidInput(m) => Self::InvalidInput(m),
            VideoPipelineError::Router(m) => Self::Router(m),
            VideoPipelineError::NoOutput => Self::NoOutput,
            VideoPipelineError::Download(m) => Self::Download(m),
            VideoPipelineError::Cache(m) => Self::Cache(m),
        }
    }
}

#[tauri::command]
pub async fn generate_video_from_text(
    input: VideoTextInput,
    state: State<'_, VideoPipelineState>,
) -> Result<VideoResult, VideoIpcError> {
    state.0.generate_from_text(input).await.map_err(Into::into)
}

#[tauri::command]
pub async fn generate_video_from_image(
    input: VideoImageInput,
    state: State<'_, VideoPipelineState>,
) -> Result<VideoResult, VideoIpcError> {
    state.0.generate_from_image(input).await.map_err(Into::into)
}
