use std::time::Duration;

use thiserror::Error;

use super::models::Provider;

/// Errors returned by the router itself.
#[derive(Debug, Error)]
pub enum RouterError {
    #[error("provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("all fallback models failed{last}",
        last = .last.as_ref().map(|s| format!(" (last: {s})")).unwrap_or_default())]
    AllFallbacksFailed {
        /// Display of the last provider error observed, for diagnostics.
        last: Option<String>,
    },

    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),
}

/// Errors returned by provider clients. The router uses the variant tags to
/// decide whether to retry (same model) or fall back (next model).
#[derive(Debug, Error, Clone)]
pub enum ProviderError {
    #[error("no client registered for provider {0:?}")]
    NoClient(Provider),

    /// Transient: retry the same model up to `max_attempts`. Example: 5xx,
    /// network hiccup, connection reset.
    #[error("transient: {0}")]
    Transient(String),

    /// Rate-limited: retriable but callers may want to wait the advertised
    /// `Retry-After` duration before the next attempt.
    #[error("rate limited: retry in {0:?}")]
    RateLimited(Duration),

    /// Timeout: treated as transient by default.
    #[error("request timed out")]
    Timeout,

    /// Auth / API-key problem — do NOT retry, do NOT fall back to a different
    /// model from the same provider (a different provider may still work).
    #[error("auth: {0}")]
    Auth(String),

    /// Permanent: schema / invalid input / 4xx that won't be fixed by retrying
    /// or by switching models. Stop immediately.
    #[error("permanent: {0}")]
    Permanent(String),

    /// The client successfully submitted a remote job (POST returned 2xx + a
    /// task_id) but polling for its terminal status failed (exhausted poll
    /// attempts, etc.). The router MUST NOT retry this — retrying would
    /// create a duplicate billable job. Falling back to a different model
    /// would also double-bill since the original job is still in flight
    /// upstream. The user can re-issue the request manually if they want a
    /// fresh attempt.
    #[error("job submitted but polling failed: {0}")]
    JobAlreadySubmitted(String),
}

impl ProviderError {
    /// Same-model retry eligibility.
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            Self::Transient(_) | Self::RateLimited(_) | Self::Timeout
        )
        // JobAlreadySubmitted intentionally NOT here — the job is in flight
        // upstream; retry would double-bill.
    }

    /// Cross-model fallback eligibility — i.e. "this failure is not the user's
    /// fault; try a different model/provider".
    pub fn is_retriable_on_another_model(&self) -> bool {
        matches!(
            self,
            Self::Transient(_)
                | Self::RateLimited(_)
                | Self::Timeout
                | Self::NoClient(_)
                | Self::Auth(_)
        )
        // JobAlreadySubmitted intentionally NOT here — falling back to a
        // different model would also create a duplicate job. The user already
        // has one in flight; let it complete or be cancelled by the caller.
    }
}
