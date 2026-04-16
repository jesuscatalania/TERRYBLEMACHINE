//! Tauri command bindings for the [`KeyStore`](super::KeyStore) trait.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use super::{KeyStore, KeyStoreError};

/// Thin wrapper so Tauri can inject the store as a [`State`].
pub struct KeyStoreState(pub Arc<dyn KeyStore>);

impl KeyStoreState {
    pub fn new(store: Arc<dyn KeyStore>) -> Self {
        Self(store)
    }
}

/// Serializable error used in command results — keeps the API typed on the frontend.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "detail")]
pub enum KeyStoreIpcError {
    NotFound(String),
    InvalidService(String),
    Keychain(String),
}

impl From<KeyStoreError> for KeyStoreIpcError {
    fn from(value: KeyStoreError) -> Self {
        match value {
            KeyStoreError::NotFound(s) => Self::NotFound(s),
            KeyStoreError::InvalidService(s) => Self::InvalidService(s),
            KeyStoreError::Keychain(s) => Self::Keychain(s),
        }
    }
}

#[tauri::command]
pub fn store_api_key(
    service: String,
    key: String,
    state: State<'_, KeyStoreState>,
) -> Result<(), KeyStoreIpcError> {
    state.0.store(&service, &key).map_err(Into::into)
}

#[tauri::command]
pub fn get_api_key(
    service: String,
    state: State<'_, KeyStoreState>,
) -> Result<String, KeyStoreIpcError> {
    state.0.get(&service).map_err(Into::into)
}

#[tauri::command]
pub fn delete_api_key(
    service: String,
    state: State<'_, KeyStoreState>,
) -> Result<(), KeyStoreIpcError> {
    state.0.delete(&service).map_err(Into::into)
}

#[tauri::command]
pub fn list_api_keys(state: State<'_, KeyStoreState>) -> Result<Vec<String>, KeyStoreIpcError> {
    state.0.list().map_err(Into::into)
}
