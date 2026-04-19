use std::collections::HashMap;
use std::env;

use parking_lot::Mutex;

use super::errors::KeyStoreError;

/// Read/write interface over an API-key store.
///
/// Implementations must be thread-safe.
pub trait KeyStore: Send + Sync {
    /// Persist `key` for the given `service` identifier. Overwrites any existing entry.
    fn store(&self, service: &str, key: &str) -> Result<(), KeyStoreError>;

    /// Read the key for `service`, returning [`KeyStoreError::NotFound`] if no entry exists.
    fn get(&self, service: &str) -> Result<String, KeyStoreError>;

    /// Delete the entry for `service`. Returns Ok even if no entry existed.
    fn delete(&self, service: &str) -> Result<(), KeyStoreError>;

    /// List all known service identifiers this store currently holds a key for.
    fn list(&self) -> Result<Vec<String>, KeyStoreError>;
}

fn validate_service(service: &str) -> Result<(), KeyStoreError> {
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

// ---------------------------------------------------------------------------
// InMemoryStore
// ---------------------------------------------------------------------------

/// Process-local store backed by a [`HashMap`]. Useful for tests and non-macOS hosts.
#[derive(Default)]
pub struct InMemoryStore {
    inner: Mutex<HashMap<String, String>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl KeyStore for InMemoryStore {
    fn store(&self, service: &str, key: &str) -> Result<(), KeyStoreError> {
        validate_service(service)?;
        let mut guard = self.inner.lock();
        guard.insert(service.to_owned(), key.to_owned());
        Ok(())
    }

    fn get(&self, service: &str) -> Result<String, KeyStoreError> {
        validate_service(service)?;
        let guard = self.inner.lock();
        guard
            .get(service)
            .cloned()
            .ok_or_else(|| KeyStoreError::NotFound(service.to_owned()))
    }

    fn delete(&self, service: &str) -> Result<(), KeyStoreError> {
        validate_service(service)?;
        let mut guard = self.inner.lock();
        guard.remove(service);
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>, KeyStoreError> {
        let guard = self.inner.lock();
        let mut keys: Vec<String> = guard.keys().cloned().collect();
        keys.sort();
        Ok(keys)
    }
}

// ---------------------------------------------------------------------------
// EnvStore
// ---------------------------------------------------------------------------

/// Reads keys from environment variables with a fixed prefix.
///
/// Reading `service = "claude"` with prefix `TERRYBLEMACHINE_KEY_` resolves to
/// `TERRYBLEMACHINE_KEY_CLAUDE`. The service name is uppercased and `-`/`.` become `_`.
///
/// Writes are kept in-process only — env vars are never mutated. This keeps
/// `set_var`/`remove_var` (which are unsafe in Rust 2024) out of the hot path.
pub struct EnvStore {
    prefix: String,
    overrides: Mutex<HashMap<String, String>>,
    tombstones: Mutex<HashMap<String, ()>>,
}

impl EnvStore {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            overrides: Mutex::new(HashMap::new()),
            tombstones: Mutex::new(HashMap::new()),
        }
    }

    fn env_name(&self, service: &str) -> String {
        let normalized: String = service
            .chars()
            .map(|c| match c {
                '-' | '.' => '_',
                c => c.to_ascii_uppercase(),
            })
            .collect();
        format!("{}{}", self.prefix, normalized)
    }

    fn env_services(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (name, _) in env::vars() {
            if let Some(rest) = name.strip_prefix(&self.prefix) {
                out.push(rest.to_ascii_lowercase());
            }
        }
        out
    }
}

impl KeyStore for EnvStore {
    fn store(&self, service: &str, key: &str) -> Result<(), KeyStoreError> {
        validate_service(service)?;
        let service = service.to_owned();
        self.overrides
            .lock()
            .insert(service.clone(), key.to_owned());
        self.tombstones.lock().remove(&service);
        Ok(())
    }

    fn get(&self, service: &str) -> Result<String, KeyStoreError> {
        validate_service(service)?;
        if self.tombstones.lock().contains_key(service) {
            return Err(KeyStoreError::NotFound(service.to_owned()));
        }
        if let Some(value) = self.overrides.lock().get(service) {
            return Ok(value.clone());
        }
        env::var(self.env_name(service)).map_err(|_| KeyStoreError::NotFound(service.to_owned()))
    }

    fn delete(&self, service: &str) -> Result<(), KeyStoreError> {
        validate_service(service)?;
        self.overrides.lock().remove(service);
        self.tombstones.lock().insert(service.to_owned(), ());
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>, KeyStoreError> {
        let mut services = self.env_services();
        {
            let overrides = self.overrides.lock();
            services.extend(overrides.keys().cloned());
        }
        {
            let tombstones = self.tombstones.lock();
            services.retain(|s| !tombstones.contains_key(s));
        }
        services.sort();
        services.dedup();
        Ok(services)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- InMemoryStore ----------------------------------------------------

    #[test]
    fn in_memory_store_stores_and_retrieves() {
        let store = InMemoryStore::new();
        store.store("claude", "sk-123").unwrap();
        assert_eq!(store.get("claude").unwrap(), "sk-123");
    }

    #[test]
    fn in_memory_store_overwrites_existing() {
        let store = InMemoryStore::new();
        store.store("claude", "sk-old").unwrap();
        store.store("claude", "sk-new").unwrap();
        assert_eq!(store.get("claude").unwrap(), "sk-new");
    }

    #[test]
    fn in_memory_store_get_missing_is_not_found() {
        let store = InMemoryStore::new();
        let err = store.get("ghost").unwrap_err();
        assert!(matches!(err, KeyStoreError::NotFound(ref s) if s == "ghost"));
    }

    #[test]
    fn in_memory_store_delete_removes_entry() {
        let store = InMemoryStore::new();
        store.store("claude", "sk-123").unwrap();
        store.delete("claude").unwrap();
        assert!(matches!(
            store.get("claude").unwrap_err(),
            KeyStoreError::NotFound(_)
        ));
    }

    #[test]
    fn in_memory_store_delete_missing_is_ok() {
        let store = InMemoryStore::new();
        store.delete("ghost").unwrap();
    }

    #[test]
    fn in_memory_store_list_returns_sorted_services() {
        let store = InMemoryStore::new();
        store.store("kling", "k").unwrap();
        store.store("claude", "c").unwrap();
        store.store("fal", "f").unwrap();
        assert_eq!(store.list().unwrap(), vec!["claude", "fal", "kling"]);
    }

    #[test]
    fn in_memory_store_rejects_empty_service() {
        let store = InMemoryStore::new();
        assert!(matches!(
            store.store("", "x").unwrap_err(),
            KeyStoreError::InvalidService(_)
        ));
    }

    #[test]
    fn in_memory_store_rejects_invalid_chars() {
        let store = InMemoryStore::new();
        assert!(matches!(
            store.store("claude/evil", "x").unwrap_err(),
            KeyStoreError::InvalidService(_)
        ));
    }

    // ---- EnvStore ---------------------------------------------------------

    #[test]
    fn env_store_round_trip_via_overrides() {
        let store = EnvStore::new("TM_TEST_");
        store.store("claude", "sk-override").unwrap();
        assert_eq!(store.get("claude").unwrap(), "sk-override");
        store.delete("claude").unwrap();
        assert!(matches!(
            store.get("claude").unwrap_err(),
            KeyStoreError::NotFound(_)
        ));
    }

    #[test]
    fn env_store_overrides_beat_env_vars() {
        // SAFETY: test mutates process env — serialized by #[test] isolation is not
        // guaranteed, but this var name is unique to this test.
        let var = "TM_ENVSTORE_OVERRIDE_CLAUDE";
        // SAFETY: no other thread reads this specific var.
        unsafe { env::set_var(var, "from-env") };

        let store = EnvStore::new("TM_ENVSTORE_OVERRIDE_");
        assert_eq!(store.get("claude").unwrap(), "from-env");
        store.store("claude", "from-memory").unwrap();
        assert_eq!(store.get("claude").unwrap(), "from-memory");

        // SAFETY: cleanup.
        unsafe { env::remove_var(var) };
    }

    #[test]
    fn env_store_normalizes_service_name_to_env() {
        let store = EnvStore::new("PFX_");
        assert_eq!(store.env_name("claude"), "PFX_CLAUDE");
        assert_eq!(store.env_name("claude-vision"), "PFX_CLAUDE_VISION");
        assert_eq!(store.env_name("foo.bar"), "PFX_FOO_BAR");
    }

    #[test]
    fn env_store_list_includes_overrides() {
        let store = EnvStore::new("TM_ENVSTORE_LIST_");
        store.store("alpha", "a").unwrap();
        store.store("beta", "b").unwrap();
        let listed = store.list().unwrap();
        assert!(listed.contains(&"alpha".to_owned()));
        assert!(listed.contains(&"beta".to_owned()));
    }
}
