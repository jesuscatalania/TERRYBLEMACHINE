//! Tauri IPC commands for the taste engine.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use super::{EnrichOptions, StyleProfile, TasteEngine, TasteError};

pub struct TasteEngineState(pub Arc<TasteEngine>);

impl TasteEngineState {
    pub fn new(engine: Arc<TasteEngine>) -> Self {
        Self(engine)
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum TasteIpcError {
    Io(String),
    Analysis(String),
    Watcher(String),
    Parse(String),
}

impl From<TasteError> for TasteIpcError {
    fn from(value: TasteError) -> Self {
        match value {
            TasteError::Io(e) => Self::Io(e.to_string()),
            TasteError::Analysis(m) => Self::Analysis(m),
            TasteError::Watcher(m) => Self::Watcher(m),
            TasteError::Parse(m) => Self::Parse(m),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EnrichInput {
    pub prompt: String,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub with_negative: bool,
}

#[tauri::command]
pub async fn refresh_taste(
    state: State<'_, TasteEngineState>,
) -> Result<StyleProfile, TasteIpcError> {
    state.0.refresh().await.map_err(Into::into)
}

#[tauri::command]
pub async fn get_taste_profile(state: State<'_, TasteEngineState>) -> Result<StyleProfile, ()> {
    Ok(state.0.profile().await)
}

#[tauri::command]
pub async fn enrich_taste_prompt(
    input: EnrichInput,
    state: State<'_, TasteEngineState>,
) -> Result<String, ()> {
    let opts = EnrichOptions {
        module: input.module,
        tags: input.tags,
        with_negative: input.with_negative,
    };
    Ok(state.0.enrich(&input.prompt, &opts).await)
}

#[tauri::command]
pub async fn get_negative_prompt(state: State<'_, TasteEngineState>) -> Result<String, ()> {
    Ok(state.0.negative_prompt().await)
}
