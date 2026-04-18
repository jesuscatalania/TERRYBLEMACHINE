//! Types for the Remotion render pipeline.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct RemotionInput {
    pub composition: String,
    pub props: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemotionResult {
    pub output_path: PathBuf,
    pub composition: String,
}

#[derive(Debug, Error)]
pub enum RemotionError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("render process failed: {0}")]
    Process(String),
    #[error("cache error: {0}")]
    Cache(String),
}
