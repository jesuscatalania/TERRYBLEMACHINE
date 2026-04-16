//! API key management.
//!
//! Provides a [`KeyStore`] trait with three implementations:
//! - [`InMemoryStore`] — process-local HashMap, used for tests.
//! - [`EnvStore`] — reads environment variables with a configured prefix (dev fallback).
//! - [`KeychainStore`] — macOS Keychain via `security-framework` (production).
//!
//! Use [`default_store`] to pick the right implementation based on runtime env.

mod errors;
mod stores;

#[cfg(target_os = "macos")]
mod keychain_store;

pub mod commands;

pub use errors::KeyStoreError;
pub use stores::{EnvStore, InMemoryStore, KeyStore};

#[cfg(target_os = "macos")]
pub use keychain_store::KeychainStore;

/// Env var that forces the env-var fallback implementation.
pub const FALLBACK_ENV: &str = "TERRYBLEMACHINE_KEY_FALLBACK";

/// Env var prefix used by [`EnvStore`].
pub const DEFAULT_ENV_PREFIX: &str = "TERRYBLEMACHINE_KEY_";

/// Default Keychain service identifier used by [`KeychainStore`].
#[cfg(target_os = "macos")]
pub const DEFAULT_KEYCHAIN_SERVICE: &str = "com.terryblemachine.app";

/// Returns the preferred [`KeyStore`] for the current environment.
///
/// If `TERRYBLEMACHINE_KEY_FALLBACK` is set (any value), returns the [`EnvStore`] fallback.
/// Otherwise returns the platform store (Keychain on macOS, [`InMemoryStore`] elsewhere so
/// tests running on other platforms don't fail).
pub fn default_store() -> Box<dyn KeyStore> {
    if std::env::var(FALLBACK_ENV).is_ok() {
        return Box::new(EnvStore::new(DEFAULT_ENV_PREFIX));
    }

    #[cfg(target_os = "macos")]
    {
        Box::new(KeychainStore::new(DEFAULT_KEYCHAIN_SERVICE))
    }

    #[cfg(not(target_os = "macos"))]
    {
        Box::new(InMemoryStore::new())
    }
}
