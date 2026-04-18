//! Tauri IPC command + state for the storyboard generator.

use std::sync::Arc;

use serde::Serialize;
use thiserror::Error;

use super::types::{Storyboard, StoryboardError, StoryboardGenerator, StoryboardInput};

pub struct StoryboardGeneratorState(pub Arc<dyn StoryboardGenerator>);

impl StoryboardGeneratorState {
    pub fn new(g: Arc<dyn StoryboardGenerator>) -> Self {
        Self(g)
    }
}

#[derive(Debug, Serialize, Error)]
#[serde(tag = "kind", content = "detail", rename_all = "kebab-case")]
pub enum StoryboardIpcError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("router error: {0}")]
    Router(String),
    #[error("parse error: {0}")]
    Parse(String),
}

impl From<StoryboardError> for StoryboardIpcError {
    fn from(e: StoryboardError) -> Self {
        match e {
            StoryboardError::InvalidInput(m) => Self::InvalidInput(m),
            StoryboardError::Router(m) => Self::Router(m),
            StoryboardError::Parse(m) => Self::Parse(m),
        }
    }
}

#[tauri::command]
pub async fn generate_storyboard(
    state: tauri::State<'_, StoryboardGeneratorState>,
    input: StoryboardInput,
) -> Result<Storyboard, StoryboardIpcError> {
    state.0.generate(input).await.map_err(Into::into)
}
