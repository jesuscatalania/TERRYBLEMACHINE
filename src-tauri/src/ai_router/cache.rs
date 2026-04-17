//! Semantic response cache.
//!
//! Keys are SHA-256 hex digests of (prompt ∥ model ∥ params). The cache is an
//! LRU with a hard entry cap and a per-entry TTL. Hits bump `last_accessed`
//! and count in [`CacheStats`]. Disk persistence writes the whole map as JSON.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use super::models::{AiResponse, Model};

pub const DEFAULT_MAX_ENTRIES: usize = 500;
pub const DEFAULT_TTL_SECONDS: u64 = 24 * 60 * 60; // 24h

/// Tunable cache parameters.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_entries: usize,
    pub ttl: Duration,
    /// If `Some`, [`SemanticCache::save_to_disk`] writes here and
    /// [`SemanticCache::load_from_disk`] reads from here.
    pub persistence_path: Option<PathBuf>,
}

impl CacheConfig {
    pub fn in_memory() -> Self {
        Self {
            max_entries: DEFAULT_MAX_ENTRIES,
            ttl: Duration::from_secs(DEFAULT_TTL_SECONDS),
            persistence_path: None,
        }
    }

    pub fn on_disk(path: PathBuf) -> Self {
        Self {
            persistence_path: Some(path),
            ..Self::in_memory()
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self::in_memory()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheEntry {
    pub response: AiResponse,
    pub inserted_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
}

/// Frontend-facing snapshot. Ratios are computed at read time.
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size: usize,
    /// SHA-256 of the oldest entry (by insertion time) — useful for eviction diagnostics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_inserted_at: Option<DateTime<Utc>>,
    /// Hits / (hits + misses). Zero when no lookups yet.
    pub hit_ratio: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct CounterState {
    hits: u64,
    misses: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedCache {
    entries: HashMap<String, CacheEntry>,
    counters: CounterState,
}

pub struct SemanticCache {
    config: CacheConfig,
    inner: Mutex<Inner>,
}

struct Inner {
    entries: HashMap<String, CacheEntry>,
    counters: CounterState,
}

impl SemanticCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            inner: Mutex::new(Inner {
                entries: HashMap::new(),
                counters: CounterState::default(),
            }),
        }
    }

    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Deterministic SHA-256 key for (prompt + model + params).
    pub fn key(prompt: &str, model: Model, params: &serde_json::Value) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        hasher.update([0]);
        hasher.update(
            serde_json::to_string(&model)
                .expect("Model serializes")
                .as_bytes(),
        );
        hasher.update([0]);
        hasher.update(serde_json::to_string(params).unwrap_or_default().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Look up a response. Returns `None` on miss or expired entry. Bumps
    /// `last_accessed_at` on hit.
    pub async fn get(&self, key: &str) -> Option<AiResponse> {
        self.get_at(key, Utc::now()).await
    }

    /// Look up with an explicit "now" — makes TTL tests deterministic.
    pub async fn get_at(&self, key: &str, now: DateTime<Utc>) -> Option<AiResponse> {
        let mut inner = self.inner.lock().await;
        let expired = match inner.entries.get(key) {
            Some(entry) => is_expired(entry, now, self.config.ttl),
            None => {
                inner.counters.misses += 1;
                return None;
            }
        };
        if expired {
            inner.entries.remove(key);
            inner.counters.misses += 1;
            return None;
        }

        let entry = inner.entries.get_mut(key).expect("just checked present");
        entry.last_accessed_at = now;
        let response = entry.response.clone();
        inner.counters.hits += 1;
        // mark as cached on the copy we return
        let mut response = response;
        response.cached = true;
        Some(response)
    }

    /// Insert or overwrite an entry. Evicts the LRU victim when at capacity.
    pub async fn put(&self, key: String, response: AiResponse) {
        self.put_at(key, response, Utc::now()).await;
    }

    /// Insert with an explicit timestamp.
    pub async fn put_at(&self, key: String, response: AiResponse, now: DateTime<Utc>) {
        let mut inner = self.inner.lock().await;
        inner.entries.insert(
            key,
            CacheEntry {
                response,
                inserted_at: now,
                last_accessed_at: now,
            },
        );
        // Evict LRU while we're above capacity.
        while inner.entries.len() > self.config.max_entries {
            if let Some(evict_key) = find_lru_key(&inner.entries) {
                inner.entries.remove(&evict_key);
            } else {
                break;
            }
        }
    }

    pub async fn len(&self) -> usize {
        self.inner.lock().await.entries.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.entries.is_empty()
    }

    pub async fn clear(&self) {
        let mut inner = self.inner.lock().await;
        inner.entries.clear();
        inner.counters = CounterState::default();
    }

    pub async fn stats(&self) -> CacheStats {
        let inner = self.inner.lock().await;
        let (oldest_key, oldest_inserted_at) = inner
            .entries
            .iter()
            .min_by_key(|(_, e)| e.inserted_at)
            .map(|(k, e)| (Some(k.clone()), Some(e.inserted_at)))
            .unwrap_or((None, None));
        let total = inner.counters.hits + inner.counters.misses;
        let hit_ratio = if total == 0 {
            0.0
        } else {
            inner.counters.hits as f64 / total as f64
        };
        CacheStats {
            hits: inner.counters.hits,
            misses: inner.counters.misses,
            size: inner.entries.len(),
            oldest_key,
            oldest_inserted_at,
            hit_ratio,
        }
    }

    /// Write cache contents to disk atomically.
    pub async fn save_to_disk(&self) -> std::io::Result<()> {
        let Some(path) = self.config.persistence_path.clone() else {
            return Ok(());
        };
        let snapshot = {
            let inner = self.inner.lock().await;
            PersistedCache {
                entries: inner.entries.clone(),
                counters: inner.counters.clone(),
            }
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&snapshot)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Read cache from disk. If the file doesn't exist, returns an empty
    /// cache with the same config. Expired entries are dropped on load.
    pub async fn load_from_disk(config: CacheConfig) -> std::io::Result<Self> {
        let cache = Self::new(config.clone());
        let Some(path) = config.persistence_path.as_ref() else {
            return Ok(cache);
        };
        if !Path::new(path).exists() {
            return Ok(cache);
        }
        let bytes = std::fs::read(path)?;
        let persisted: PersistedCache = serde_json::from_slice(&bytes)?;
        let now = Utc::now();
        {
            let mut inner = cache.inner.lock().await;
            for (k, v) in persisted.entries {
                if !is_expired(&v, now, config.ttl) {
                    inner.entries.insert(k, v);
                }
            }
            inner.counters = persisted.counters;
        }
        Ok(cache)
    }
}

fn is_expired(entry: &CacheEntry, now: DateTime<Utc>, ttl: Duration) -> bool {
    let lifetime = now - entry.inserted_at;
    match lifetime.to_std() {
        Ok(elapsed) => elapsed >= ttl,
        // negative (now earlier than insertion) → treat as not expired
        Err(_) => false,
    }
}

fn find_lru_key(entries: &HashMap<String, CacheEntry>) -> Option<String> {
    entries
        .iter()
        .min_by_key(|(_, e)| e.last_accessed_at)
        .map(|(k, _)| k.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;
    use serde_json::json;

    fn fake_response(id: &str) -> AiResponse {
        AiResponse {
            request_id: id.into(),
            model: Model::ClaudeSonnet,
            output: json!({ "text": "hi" }),
            cost_cents: None,
            cached: false,
        }
    }

    #[test]
    fn key_is_deterministic() {
        let a = SemanticCache::key("prompt", Model::ClaudeSonnet, &json!({ "k": 1 }));
        let b = SemanticCache::key("prompt", Model::ClaudeSonnet, &json!({ "k": 1 }));
        assert_eq!(a, b);
        assert_eq!(a.len(), 64); // 256 bits hex
    }

    #[test]
    fn key_differs_for_different_inputs() {
        let base = SemanticCache::key("prompt", Model::ClaudeSonnet, &json!({}));
        assert_ne!(
            base,
            SemanticCache::key("prompt!", Model::ClaudeSonnet, &json!({}))
        );
        assert_ne!(
            base,
            SemanticCache::key("prompt", Model::ClaudeOpus, &json!({}))
        );
        assert_ne!(
            base,
            SemanticCache::key("prompt", Model::ClaudeSonnet, &json!({ "x": 1 }))
        );
    }

    #[tokio::test]
    async fn get_on_empty_records_miss() {
        let cache = SemanticCache::new(CacheConfig::in_memory());
        assert!(cache.get("unknown").await.is_none());
        let stats = cache.stats().await;
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn put_then_get_is_a_hit_and_marks_cached_flag() {
        let cache = SemanticCache::new(CacheConfig::in_memory());
        cache.put("k".into(), fake_response("r1")).await;
        let hit = cache.get("k").await.expect("cache hit");
        assert_eq!(hit.request_id, "r1");
        assert!(hit.cached, "returned response should carry cached=true");
        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.size, 1);
        assert!((stats.hit_ratio - 1.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn ttl_expires_old_entries_on_get() {
        let cfg = CacheConfig {
            ttl: Duration::from_secs(60),
            ..CacheConfig::in_memory()
        };
        let cache = SemanticCache::new(cfg);
        let t0 = Utc::now();
        cache.put_at("k".into(), fake_response("r1"), t0).await;
        // Inside TTL
        assert!(cache
            .get_at("k", t0 + ChronoDuration::seconds(30))
            .await
            .is_some());
        // After TTL
        assert!(cache
            .get_at("k", t0 + ChronoDuration::seconds(120))
            .await
            .is_none());
        // Expired entry is evicted
        assert_eq!(cache.len().await, 0);
    }

    #[tokio::test]
    async fn lru_evicts_least_recently_accessed() {
        let cfg = CacheConfig {
            max_entries: 2,
            ..CacheConfig::in_memory()
        };
        let cache = SemanticCache::new(cfg);
        let t0 = Utc::now();
        cache.put_at("a".into(), fake_response("a"), t0).await;
        cache
            .put_at(
                "b".into(),
                fake_response("b"),
                t0 + ChronoDuration::seconds(1),
            )
            .await;
        // Touch a so it becomes most recent
        let _ = cache.get_at("a", t0 + ChronoDuration::seconds(2)).await;
        // Insert c — "b" should be evicted (LRU)
        cache
            .put_at(
                "c".into(),
                fake_response("c"),
                t0 + ChronoDuration::seconds(3),
            )
            .await;
        assert_eq!(cache.len().await, 2);
        assert!(cache
            .get_at("a", t0 + ChronoDuration::seconds(4))
            .await
            .is_some());
        assert!(cache
            .get_at("c", t0 + ChronoDuration::seconds(5))
            .await
            .is_some());
        assert!(cache
            .get_at("b", t0 + ChronoDuration::seconds(6))
            .await
            .is_none());
    }

    #[tokio::test]
    async fn stats_report_oldest_entry() {
        let cache = SemanticCache::new(CacheConfig::in_memory());
        let t0 = Utc::now();
        cache.put_at("a".into(), fake_response("a"), t0).await;
        cache
            .put_at(
                "b".into(),
                fake_response("b"),
                t0 + ChronoDuration::seconds(10),
            )
            .await;
        let stats = cache.stats().await;
        assert_eq!(stats.size, 2);
        assert_eq!(stats.oldest_key.as_deref(), Some("a"));
        assert_eq!(stats.oldest_inserted_at, Some(t0));
    }

    #[tokio::test]
    async fn clear_empties_everything() {
        let cache = SemanticCache::new(CacheConfig::in_memory());
        cache.put("k".into(), fake_response("r")).await;
        let _ = cache.get("k").await;
        cache.clear().await;
        let stats = cache.stats().await;
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.size, 0);
    }

    #[tokio::test]
    async fn hit_ratio_is_zero_when_no_lookups() {
        let cache = SemanticCache::new(CacheConfig::in_memory());
        assert_eq!(cache.stats().await.hit_ratio, 0.0);
    }

    #[tokio::test]
    async fn save_and_load_round_trip_preserves_entries_and_counters() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("cache.json");
        let cfg = CacheConfig::on_disk(path.clone());

        // Seed
        {
            let cache = SemanticCache::new(cfg.clone());
            cache.put("k".into(), fake_response("r1")).await;
            let _ = cache.get("k").await; // hit +1
            let _ = cache.get("missing").await; // miss +1
            cache.save_to_disk().await.unwrap();
        }

        // Reload
        let restored = SemanticCache::load_from_disk(cfg).await.unwrap();
        let stats = restored.stats().await;
        assert_eq!(stats.size, 1);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!(restored.get("k").await.is_some());
    }

    #[tokio::test]
    async fn load_missing_file_returns_empty_cache() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("nope.json");
        let cfg = CacheConfig::on_disk(path);
        let cache = SemanticCache::load_from_disk(cfg).await.unwrap();
        assert_eq!(cache.len().await, 0);
    }

    #[tokio::test]
    async fn load_drops_entries_that_are_already_expired() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("old.json");
        // Tiny TTL so the entry we seed is dead by the time we reload.
        let cfg = CacheConfig {
            ttl: Duration::from_millis(1),
            ..CacheConfig::on_disk(path.clone())
        };
        {
            let cache = SemanticCache::new(cfg.clone());
            cache.put("k".into(), fake_response("r")).await;
            cache.save_to_disk().await.unwrap();
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
        let restored = SemanticCache::load_from_disk(cfg).await.unwrap();
        assert_eq!(restored.len().await, 0);
    }

    #[tokio::test]
    async fn save_without_persistence_path_is_noop() {
        let cache = SemanticCache::new(CacheConfig::in_memory());
        cache.put("k".into(), fake_response("r")).await;
        // Should succeed without writing anywhere.
        cache.save_to_disk().await.unwrap();
    }
}
