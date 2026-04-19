//! Cross-provider helpers: rate limiter, HTTP error mapping, key resolution.

use std::sync::Arc;
use std::time::Duration;

use reqwest::StatusCode;
use tokio::sync::Semaphore;
use tokio::time::sleep;

use crate::ai_router::ProviderError;
use crate::keychain::{KeyStore, KeyStoreError};

// ─── Rate limiter ─────────────────────────────────────────────────────

/// Simple per-second token bucket.
///
/// `max_per_sec` permits are issued at process start. The `replenisher` task
/// refills the bucket every second. `acquire()` blocks (async) when empty.
/// Refill is silently skipped if the limiter has been dropped.
#[derive(Clone)]
pub struct RateLimiter {
    sem: Arc<Semaphore>,
    capacity: usize,
    max_per_sec: usize,
    /// Guard so the refill task spawns exactly once, lazily on first acquire.
    /// Synchronous `OnceLock` is fine here because `tokio::spawn` is itself
    /// sync (returns a `JoinHandle`); the `get_or_init` closure just needs to
    /// run from within a Tokio runtime, which `acquire` (the only caller)
    /// guarantees.
    refill_started: Arc<std::sync::OnceLock<()>>,
}

impl RateLimiter {
    /// Construct a limiter. The refill task is **deferred** until first
    /// `acquire()` so this constructor is safe to call from non-async,
    /// non-runtime contexts (e.g. `lib.rs::run` before Tauri starts the
    /// Tokio runtime).
    pub fn new(max_per_sec: usize) -> Self {
        Self {
            sem: Arc::new(Semaphore::new(max_per_sec)),
            capacity: max_per_sec,
            max_per_sec,
            refill_started: Arc::new(std::sync::OnceLock::new()),
        }
    }

    /// Lower-cost variant: no refill task, permits never come back. Useful in
    /// tests.
    pub fn unlimited() -> Self {
        Self {
            sem: Arc::new(Semaphore::new(usize::MAX >> 4)),
            capacity: usize::MAX >> 4,
            max_per_sec: usize::MAX >> 4,
            // OnceLock present but never initialized — `acquire` on the
            // unlimited variant doesn't need a refill task at all.
            refill_started: Arc::new(std::sync::OnceLock::new()),
        }
    }

    /// Spawn the refill task on first call. Idempotent via `OnceLock`.
    /// Must be called from within a Tokio runtime (guaranteed by `acquire`).
    fn ensure_refill_started(&self) {
        self.refill_started.get_or_init(|| {
            let replenisher = self.sem.clone();
            let max_per_sec = self.max_per_sec;
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(Duration::from_secs(1));
                ticker.tick().await; // skip first immediate tick
                loop {
                    ticker.tick().await;
                    let deficit = max_per_sec.saturating_sub(replenisher.available_permits());
                    if deficit > 0 {
                        replenisher.add_permits(deficit);
                    }
                    if Arc::strong_count(&replenisher) <= 1 {
                        break;
                    }
                }
            });
        });
    }

    pub async fn acquire(&self) {
        self.ensure_refill_started();
        let permit = self.sem.clone().acquire_owned().await;
        // We don't hold the permit — the refill task replaces it.
        drop(permit);
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

// ─── HTTP error mapping ──────────────────────────────────────────────

/// Classify an HTTP response status into a [`ProviderError`] variant.
///
/// - 5xx → `Transient` (retry same model)
/// - 429 → `RateLimited` (retry with backoff)
/// - 408 / request timeout equivalents → `Timeout`
/// - 401 / 403 → `Auth`
/// - other 4xx → `Permanent`
pub fn map_http_error(status: StatusCode, body_snippet: &str) -> ProviderError {
    let snippet = truncate(body_snippet, 240);
    match status.as_u16() {
        401 | 403 => ProviderError::Auth(snippet),
        408 => ProviderError::Timeout,
        429 => ProviderError::RateLimited(Duration::from_secs(1)),
        s if (500..600).contains(&s) => ProviderError::Transient(format!("HTTP {s}: {snippet}")),
        s if (400..500).contains(&s) => ProviderError::Permanent(format!("HTTP {s}: {snippet}")),
        s => ProviderError::Transient(format!("unexpected HTTP {s}: {snippet}")),
    }
}

/// Map a raw `reqwest` error to `ProviderError`. Network timeouts and
/// connection issues are treated as transient.
pub fn map_reqwest_error(err: reqwest::Error) -> ProviderError {
    if err.is_timeout() {
        ProviderError::Timeout
    } else {
        ProviderError::Transient(err.to_string())
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_owned()
    } else {
        let mut out = s[..max].to_owned();
        out.push('…');
        out
    }
}

// ─── Key resolution ──────────────────────────────────────────────────

/// Look up a provider key from the keychain. Missing keys surface as
/// [`ProviderError::Auth`] so the router treats them as "try another
/// provider" rather than "retry".
pub fn get_api_key(store: &dyn KeyStore, service: &str) -> Result<String, ProviderError> {
    store.get(service).map_err(|e| match e {
        KeyStoreError::NotFound(s) => ProviderError::Auth(format!("no key for {s}")),
        other => ProviderError::Auth(other.to_string()),
    })
}

// ─── Retry-after header parsing ──────────────────────────────────────

/// Parse a `Retry-After` header. Supports seconds values only (HTTP-date
/// form falls back to 1 second). Returned for future use; [`map_http_error`]
/// currently returns a constant 1-second delay for 429s.
pub fn parse_retry_after(value: &str) -> Duration {
    value
        .trim()
        .parse::<u64>()
        .map(Duration::from_secs)
        .unwrap_or_else(|_| Duration::from_secs(1))
}

/// Sleep a while without blocking the runtime. Exposed for reuse in clients.
pub async fn short_sleep(ms: u64) {
    sleep(Duration::from_millis(ms)).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_http_error_5xx_is_transient() {
        let e = map_http_error(StatusCode::BAD_GATEWAY, "upstream blew up");
        assert!(matches!(e, ProviderError::Transient(_)));
    }

    #[test]
    fn map_http_error_429_is_rate_limited() {
        let e = map_http_error(StatusCode::TOO_MANY_REQUESTS, "slow down");
        assert!(matches!(e, ProviderError::RateLimited(_)));
    }

    #[test]
    fn map_http_error_401_is_auth() {
        let e = map_http_error(StatusCode::UNAUTHORIZED, "bad key");
        assert!(matches!(e, ProviderError::Auth(_)));
    }

    #[test]
    fn map_http_error_403_is_auth() {
        let e = map_http_error(StatusCode::FORBIDDEN, "nope");
        assert!(matches!(e, ProviderError::Auth(_)));
    }

    #[test]
    fn map_http_error_400_is_permanent() {
        let e = map_http_error(StatusCode::BAD_REQUEST, "schema error");
        assert!(matches!(e, ProviderError::Permanent(_)));
    }

    #[test]
    fn parse_retry_after_accepts_seconds() {
        assert_eq!(parse_retry_after("3"), Duration::from_secs(3));
    }

    #[test]
    fn parse_retry_after_falls_back_to_one_second() {
        assert_eq!(
            parse_retry_after("Thu, 01 Jan 2026 00:00:00 GMT"),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn truncate_clamps_long_strings() {
        let s = "a".repeat(500);
        let out = truncate(&s, 10);
        assert!(out.len() <= 20);
        assert!(out.ends_with('…'));
    }

    #[tokio::test]
    async fn unlimited_rate_limiter_never_blocks() {
        let rl = RateLimiter::unlimited();
        for _ in 0..1000 {
            rl.acquire().await;
        }
    }
}
