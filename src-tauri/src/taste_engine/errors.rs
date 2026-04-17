use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TasteError {
    #[error("meingeschmack/ I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("image analysis failed: {0}")]
    Analysis(String),

    #[error("watcher error: {0}")]
    Watcher(String),

    #[error("parse error: {0}")]
    Parse(String),
}
