//! Tauri IPC commands for project management.

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use super::{default_root, FileProjectStore, NewProject, Project, ProjectError};

/// Wrapper state so Tauri can inject the store.
pub struct ProjectStoreState {
    pub root: Arc<PathBuf>,
}

impl ProjectStoreState {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root: Arc::new(root),
        }
    }

    pub fn store(&self) -> FileProjectStore {
        FileProjectStore::new((*self.root).clone())
    }
}

/// IPC-safe error mirror of [`ProjectError`].
#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum ProjectIpcError {
    NotFound(String),
    InvalidName(String),
    Io(String),
    Serde(String),
}

impl From<ProjectError> for ProjectIpcError {
    fn from(value: ProjectError) -> Self {
        match value {
            ProjectError::NotFound(s) => Self::NotFound(s),
            ProjectError::InvalidName(s) => Self::InvalidName(s),
            ProjectError::Io(e) => Self::Io(e.to_string()),
            ProjectError::Serde(e) => Self::Serde(e.to_string()),
        }
    }
}

#[tauri::command]
pub fn create_project(
    input: NewProject,
    state: State<'_, ProjectStoreState>,
) -> Result<Project, ProjectIpcError> {
    state.store().create(input).map_err(Into::into)
}

#[tauri::command]
pub fn open_project(
    id: String,
    state: State<'_, ProjectStoreState>,
) -> Result<Project, ProjectIpcError> {
    state.store().open(&id).map_err(Into::into)
}

#[tauri::command]
pub fn list_projects(state: State<'_, ProjectStoreState>) -> Result<Vec<Project>, ProjectIpcError> {
    state.store().list().map_err(Into::into)
}

#[tauri::command]
pub fn delete_project(
    id: String,
    state: State<'_, ProjectStoreState>,
) -> Result<(), ProjectIpcError> {
    state.store().delete(&id).map_err(Into::into)
}

#[tauri::command]
pub fn projects_root(state: State<'_, ProjectStoreState>) -> String {
    state.root.to_string_lossy().into_owned()
}

/// Resolve the default projects root using the app's document directory.
///
/// Falls back to the current working directory if the documents dir cannot be
/// resolved (shouldn't happen on macOS but guarded for safety).
pub fn resolve_default_root(app: &AppHandle) -> PathBuf {
    match app.path().document_dir() {
        Ok(docs) => default_root(docs),
        Err(_) => std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("TERRYBLEMACHINE")
            .join("projects"),
    }
}
