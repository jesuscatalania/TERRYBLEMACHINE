//! Tauri IPC commands for per-project undo/redo history persistence.
//!
//! History lives next to `project.json` as `history.json`. Reads return an
//! empty-stacks payload when the file does not exist (fresh projects). Writes
//! overwrite unconditionally — callers serialise the stacks client-side.

use std::path::PathBuf;

use super::commands::ProjectIpcError;

const EMPTY_HISTORY: &str = "{\"past\":[],\"future\":[]}";
const HISTORY_FILE: &str = "history.json";

/// Reads `history.json` from the given project directory.
///
/// Returns the serialised stacks as a raw JSON string. Missing files are
/// treated as "no history yet" and resolve to an empty-stacks payload rather
/// than an error, so a brand-new project opens cleanly.
#[tauri::command]
pub fn read_project_history(path: PathBuf) -> Result<String, ProjectIpcError> {
    let file = path.join(HISTORY_FILE);
    if !file.exists() {
        return Ok(EMPTY_HISTORY.to_string());
    }
    std::fs::read_to_string(&file).map_err(|e| ProjectIpcError::Io(e.to_string()))
}

/// Writes `history.json` into the given project directory.
///
/// The caller is responsible for producing a valid JSON payload — this
/// command does not validate the shape (history is opaque on the backend).
#[tauri::command]
pub fn write_project_history(path: PathBuf, json: String) -> Result<(), ProjectIpcError> {
    let file = path.join(HISTORY_FILE);
    std::fs::write(&file, json).map_err(|e| ProjectIpcError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn read_returns_empty_stacks_when_file_missing() {
        let dir = tempdir().unwrap();
        let raw = read_project_history(dir.path().to_path_buf()).unwrap();
        assert_eq!(raw, EMPTY_HISTORY);
    }

    #[test]
    fn write_then_read_round_trips() {
        let dir = tempdir().unwrap();
        let payload = r#"{"past":[{"label":"x","timestamp":"2026-04-17T00:00:00Z"}],"future":[]}"#;
        write_project_history(dir.path().to_path_buf(), payload.to_string()).unwrap();
        let raw = read_project_history(dir.path().to_path_buf()).unwrap();
        assert_eq!(raw, payload);
    }

    #[test]
    fn write_overwrites_existing_file() {
        let dir = tempdir().unwrap();
        write_project_history(dir.path().to_path_buf(), "first".to_string()).unwrap();
        write_project_history(dir.path().to_path_buf(), "second".to_string()).unwrap();
        let raw = read_project_history(dir.path().to_path_buf()).unwrap();
        assert_eq!(raw, "second");
    }
}
