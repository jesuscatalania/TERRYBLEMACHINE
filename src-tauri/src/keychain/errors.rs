use thiserror::Error;

/// Errors returned by any [`KeyStore`](super::KeyStore) implementation.
#[derive(Debug, Error)]
pub enum KeyStoreError {
    #[error("no key stored for service `{0}`")]
    NotFound(String),

    #[error("invalid service identifier: {0}")]
    InvalidService(String),

    #[error("keychain error: {0}")]
    Keychain(String),
}
