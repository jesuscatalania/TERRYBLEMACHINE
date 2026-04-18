//! Tauri command + inner helper that spawns Remotion render.

use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::process::Command;

use super::types::{RemotionError, RemotionInput, RemotionResult};

pub struct RemotionState {
    /// Absolute path to the remotion/ subpackage.
    pub remotion_root: PathBuf,
}

impl RemotionState {
    pub fn new(remotion_root: PathBuf) -> Self {
        Self { remotion_root }
    }
}

fn cache_path(composition: &str, props_json: &str) -> Result<PathBuf, RemotionError> {
    let base =
        dirs::cache_dir().ok_or_else(|| RemotionError::Cache("no platform cache dir".into()))?;
    let dir = base.join("terryblemachine").join("remotion-renders");
    std::fs::create_dir_all(&dir).map_err(|e| RemotionError::Cache(e.to_string()))?;
    let mut h = Sha256::new();
    h.update(composition.as_bytes());
    h.update(props_json.as_bytes());
    let hash = format!("{:x}", h.finalize());
    Ok(dir.join(format!("{composition}-{hash}.mp4")))
}

pub async fn render_inner(
    remotion_root: &Path,
    input: &RemotionInput,
) -> Result<RemotionResult, RemotionError> {
    if input.composition.trim().is_empty() {
        return Err(RemotionError::InvalidInput("composition is empty".into()));
    }
    // Defend against shell-injection via composition: only alphanumeric/dash/underscore
    if !input
        .composition
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(RemotionError::InvalidInput(
            "composition must be alphanumeric/-/_".into(),
        ));
    }
    let props_json = input.props.to_string();
    let output_path = cache_path(&input.composition, &props_json)?;
    if output_path.exists() {
        return Ok(RemotionResult {
            output_path,
            composition: input.composition.clone(),
        });
    }
    let output = Command::new("npx")
        .current_dir(remotion_root)
        .arg("remotion")
        .arg("render")
        .arg("src/Root.tsx")
        .arg(&input.composition)
        .arg(&output_path)
        .arg(format!("--props={props_json}"))
        .output()
        .await
        .map_err(|e| RemotionError::Process(format!("spawn: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RemotionError::Process(format!(
            "remotion render failed: {stderr}"
        )));
    }
    if !output_path.exists() {
        return Err(RemotionError::Process(
            "remotion render completed but output not found".into(),
        ));
    }
    Ok(RemotionResult {
        output_path,
        composition: input.composition.clone(),
    })
}

#[derive(Debug, Serialize, Error)]
#[serde(tag = "kind", content = "detail", rename_all = "kebab-case")]
pub enum RemotionIpcError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("process error: {0}")]
    Process(String),
    #[error("cache error: {0}")]
    Cache(String),
}

impl From<RemotionError> for RemotionIpcError {
    fn from(e: RemotionError) -> Self {
        match e {
            RemotionError::InvalidInput(m) => Self::InvalidInput(m),
            RemotionError::Process(m) => Self::Process(m),
            RemotionError::Cache(m) => Self::Cache(m),
        }
    }
}

#[tauri::command]
pub async fn render_remotion(
    state: tauri::State<'_, RemotionState>,
    input: RemotionInput,
) -> Result<RemotionResult, RemotionIpcError> {
    render_inner(&state.remotion_root, &input)
        .await
        .map_err(Into::into)
}
