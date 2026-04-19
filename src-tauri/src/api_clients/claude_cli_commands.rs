//! Tauri commands for the Claude CLI bridge.
//!
//! Three glue commands that the Settings UI calls:
//! - [`detect_claude_cli`] — returns the absolute path to the `claude` CLI
//!   binary if one is installed, or `None`.
//! - [`get_claude_transport`] — returns the persisted transport selection
//!   (`"auto"` | `"api"` | `"cli"`). Defaults to `"auto"` when nothing is
//!   stored or an invalid value is stored.
//! - [`set_claude_transport`] — persists the user's transport choice into
//!   the keychain under a reserved meta key.
//!
//! The keystore entry is keyed with underscores (`__claude_transport__`) so
//! it can't collide with any real provider service id.

use tauri::State;

use crate::keychain::commands::{KeyStoreIpcError, KeyStoreState};

/// Reserved keystore entry used to persist the user's Claude transport
/// preference. Leading/trailing underscores keep it out of the provider
/// namespace (`claude`, `kling`, …).
pub const TRANSPORT_META_KEY: &str = "__claude_transport__";

/// Only these three values are valid transport selections.
const VALID_TRANSPORTS: &[&str] = &["auto", "api", "cli"];

/// Return the absolute path to the `claude` CLI binary, if one is found on
/// this machine. Never throws — a missing binary is represented by `None`.
#[tauri::command]
pub fn detect_claude_cli() -> Option<String> {
    super::claude_cli::discovery::detect_claude_binary()
        .map(|p| p.to_string_lossy().into_owned())
}

/// Read the persisted Claude transport selection. Defaults to `"auto"` when
/// nothing is stored OR when an out-of-range value has somehow been written.
#[tauri::command]
pub fn get_claude_transport(state: State<'_, KeyStoreState>) -> String {
    match state.0.get(TRANSPORT_META_KEY) {
        Ok(value) if VALID_TRANSPORTS.contains(&value.as_str()) => value,
        _ => "auto".to_string(),
    }
}

/// Persist the user's Claude transport choice. Rejects any value that is not
/// one of `"auto"`, `"api"`, `"cli"` with a typed `InvalidService` error
/// (the keystore IPC error shape already round-trips cleanly to the UI).
#[tauri::command]
pub fn set_claude_transport(
    transport: String,
    state: State<'_, KeyStoreState>,
) -> Result<(), KeyStoreIpcError> {
    if !VALID_TRANSPORTS.contains(&transport.as_str()) {
        return Err(KeyStoreIpcError::InvalidService(format!(
            "invalid transport {transport:?} — expected one of auto|api|cli"
        )));
    }
    state
        .0
        .store(TRANSPORT_META_KEY, &transport)
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keychain::{InMemoryStore, KeyStore};
    use std::sync::Arc;

    fn make_store() -> Arc<dyn KeyStore> {
        Arc::new(InMemoryStore::new())
    }

    #[test]
    fn defaults_to_auto_when_nothing_stored() {
        let store = make_store();
        // Exercise the store path directly — `#[tauri::command]` wrappers
        // need the Tauri runtime, so we test the logic via the inner store.
        let value = match store.get(TRANSPORT_META_KEY) {
            Ok(v) if VALID_TRANSPORTS.contains(&v.as_str()) => v,
            _ => "auto".to_string(),
        };
        assert_eq!(value, "auto");
    }

    #[test]
    fn returns_stored_value_when_valid() {
        let store = make_store();
        store.store(TRANSPORT_META_KEY, "cli").unwrap();
        let value = match store.get(TRANSPORT_META_KEY) {
            Ok(v) if VALID_TRANSPORTS.contains(&v.as_str()) => v,
            _ => "auto".to_string(),
        };
        assert_eq!(value, "cli");
    }

    #[test]
    fn falls_back_to_auto_for_invalid_stored_value() {
        let store = make_store();
        store.store(TRANSPORT_META_KEY, "garbage").unwrap();
        let value = match store.get(TRANSPORT_META_KEY) {
            Ok(v) if VALID_TRANSPORTS.contains(&v.as_str()) => v,
            _ => "auto".to_string(),
        };
        assert_eq!(value, "auto");
    }
}
