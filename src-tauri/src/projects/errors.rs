use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("project not found: {0}")]
    NotFound(String),

    #[error("invalid project name: {0}")]
    InvalidName(String),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}
