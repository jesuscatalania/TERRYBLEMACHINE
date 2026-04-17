//! Tauri IPC command for the exporter.

use std::path::PathBuf;

use serde::Serialize;

use super::zip_export::{export_to_zip, ExportError, ExportRequest};

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum ExportIpcError {
    InvalidRequest(String),
    Io(String),
    Zip(String),
}

impl From<ExportError> for ExportIpcError {
    fn from(value: ExportError) -> Self {
        match value {
            ExportError::Io(e) => Self::Io(e.to_string()),
            ExportError::Zip(e) => Self::Zip(e.to_string()),
            ExportError::InvalidRequest(m) => Self::InvalidRequest(m),
        }
    }
}

#[tauri::command]
pub fn export_website(request: ExportRequest) -> Result<PathBuf, ExportIpcError> {
    export_to_zip(&request).map_err(Into::into)
}
