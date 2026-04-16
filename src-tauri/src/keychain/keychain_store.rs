//! macOS Keychain-backed [`KeyStore`](super::KeyStore) implementation.
//!
//! Generic-password entries are stored as (service_prefix, service_id).
//! `list()` iterates the configured `known_services` and returns those with
//! an entry present — the Keychain search API for enumeration is awkward
//! and we always know the fixed provider set.

use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

use super::{KeyStore, KeyStoreError};

/// Default set of providers we might store API keys for.
pub const DEFAULT_KNOWN_SERVICES: &[&str] = &[
    "claude",
    "kling",
    "runway",
    "higgsfield",
    "shotstack",
    "ideogram",
    "meshy",
    "fal",
    "replicate",
];

pub struct KeychainStore {
    service_prefix: String,
    known_services: Vec<String>,
}

impl KeychainStore {
    pub fn new(service_prefix: impl Into<String>) -> Self {
        Self {
            service_prefix: service_prefix.into(),
            known_services: DEFAULT_KNOWN_SERVICES
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
        }
    }

    pub fn with_known_services(
        service_prefix: impl Into<String>,
        known_services: Vec<String>,
    ) -> Self {
        Self {
            service_prefix: service_prefix.into(),
            known_services,
        }
    }
}

fn validate(service: &str) -> Result<(), KeyStoreError> {
    if service.is_empty() {
        return Err(KeyStoreError::InvalidService("empty".into()));
    }
    if !service
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(KeyStoreError::InvalidService(service.into()));
    }
    Ok(())
}

fn is_not_found(err: &security_framework::base::Error) -> bool {
    err.code() == -25300
}

impl KeyStore for KeychainStore {
    fn store(&self, service: &str, key: &str) -> Result<(), KeyStoreError> {
        validate(service)?;
        set_generic_password(&self.service_prefix, service, key.as_bytes())
            .map_err(|e| KeyStoreError::Keychain(e.to_string()))
    }

    fn get(&self, service: &str) -> Result<String, KeyStoreError> {
        validate(service)?;
        match get_generic_password(&self.service_prefix, service) {
            Ok(bytes) => String::from_utf8(bytes)
                .map_err(|e| KeyStoreError::Keychain(format!("non-utf8 password: {e}"))),
            Err(e) if is_not_found(&e) => Err(KeyStoreError::NotFound(service.to_owned())),
            Err(e) => Err(KeyStoreError::Keychain(e.to_string())),
        }
    }

    fn delete(&self, service: &str) -> Result<(), KeyStoreError> {
        validate(service)?;
        match delete_generic_password(&self.service_prefix, service) {
            Ok(()) => Ok(()),
            Err(e) if is_not_found(&e) => Ok(()),
            Err(e) => Err(KeyStoreError::Keychain(e.to_string())),
        }
    }

    fn list(&self) -> Result<Vec<String>, KeyStoreError> {
        let mut present = Vec::new();
        for service in &self.known_services {
            match get_generic_password(&self.service_prefix, service) {
                Ok(_) => present.push(service.clone()),
                Err(e) if is_not_found(&e) => {}
                Err(e) => return Err(KeyStoreError::Keychain(e.to_string())),
            }
        }
        present.sort();
        Ok(present)
    }
}
