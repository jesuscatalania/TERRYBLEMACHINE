//! AI Router — routes generative requests to the right provider + model.
//!
//! This module's responsibilities:
//! - Classify incoming requests (task kind + complexity) and pick a model.
//! - Retry transient failures with exponential backoff.
//! - Fall back to alternative models when the primary is unavailable.
//! - Expose a priority queue + status to the frontend via Tauri commands.
//!
//! The actual provider clients live in `api_clients/` (built in Schritt 2.2).
//! For Schritt 2.1 we depend only on the [`AiClient`](models::AiClient) trait.

mod budget;
mod cache;
pub mod commands;
mod errors;
mod models;
mod queue;
mod router;

pub use budget::{
    cost_cents_for, BudgetLimits, BudgetManager, BudgetState, BudgetStatus, UsageEntry,
    DEFAULT_DAILY_LIMIT_CENTS, WARN_THRESHOLD,
};
pub use cache::{
    CacheConfig, CacheEntry, CacheStats, SemanticCache, DEFAULT_MAX_ENTRIES, DEFAULT_TTL_SECONDS,
};
pub use errors::{ProviderError, RouterError};
pub use models::{
    AiClient, AiRequest, AiResponse, Complexity, Model, Priority, Provider, ProviderUsage, TaskKind,
};
pub use queue::{PriorityQueue, QueueStatus, QueuedRequest};
pub use router::{DefaultRoutingStrategy, RetryPolicy, RouteDecision, RoutingStrategy};

use std::collections::HashMap;
use std::sync::Arc;

use tokio::time::sleep;

/// Composes a routing strategy + client registry + retry policy + queue.
///
/// `route()` is the single entry point: classify, try primary, then fallbacks,
/// each with retry. Progress through the queue is tracked via the `queue`
/// field (observable by [`QueueStatus`](crate::ai_router::QueueStatus)).
pub struct AiRouter {
    strategy: Arc<dyn RoutingStrategy>,
    clients: HashMap<Provider, Arc<dyn AiClient>>,
    retry: RetryPolicy,
    queue: Arc<PriorityQueue>,
    cache: Arc<SemanticCache>,
    budget: Arc<BudgetManager>,
}

impl AiRouter {
    pub fn new(
        strategy: Arc<dyn RoutingStrategy>,
        clients: HashMap<Provider, Arc<dyn AiClient>>,
        retry: RetryPolicy,
        queue: Arc<PriorityQueue>,
    ) -> Self {
        Self::with_all(
            strategy,
            clients,
            retry,
            queue,
            Arc::new(SemanticCache::new(CacheConfig::in_memory())),
            Arc::new(BudgetManager::with_defaults()),
        )
    }

    pub fn with_cache(
        strategy: Arc<dyn RoutingStrategy>,
        clients: HashMap<Provider, Arc<dyn AiClient>>,
        retry: RetryPolicy,
        queue: Arc<PriorityQueue>,
        cache: Arc<SemanticCache>,
    ) -> Self {
        Self::with_all(
            strategy,
            clients,
            retry,
            queue,
            cache,
            Arc::new(BudgetManager::with_defaults()),
        )
    }

    pub fn with_all(
        strategy: Arc<dyn RoutingStrategy>,
        clients: HashMap<Provider, Arc<dyn AiClient>>,
        retry: RetryPolicy,
        queue: Arc<PriorityQueue>,
        cache: Arc<SemanticCache>,
        budget: Arc<BudgetManager>,
    ) -> Self {
        Self {
            strategy,
            clients,
            retry,
            queue,
            cache,
            budget,
        }
    }

    pub fn queue(&self) -> &Arc<PriorityQueue> {
        &self.queue
    }

    pub fn cache(&self) -> &Arc<SemanticCache> {
        &self.cache
    }

    pub fn budget(&self) -> &Arc<BudgetManager> {
        &self.budget
    }

    /// Execute a request through the router pipeline.
    ///
    /// 1. Check the [`SemanticCache`]. On hit, return immediately.
    /// 2. The [`RoutingStrategy`] picks a primary model + ordered fallbacks.
    /// 3. Each model is tried with [`RetryPolicy`]-guarded retries.
    /// 4. On success, the first successful [`AiResponse`] is returned
    ///    **and** inserted into the cache for future hits.
    /// 5. If every option exhausts its retries, [`RouterError::AllFallbacksFailed`]
    ///    is returned carrying the last provider error.
    pub async fn route(&self, request: AiRequest) -> Result<AiResponse, RouterError> {
        let decision = self.strategy.select(&request);

        let cache_key = SemanticCache::key(&request.prompt, decision.primary, &request.payload);
        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok(cached);
        }

        // Gate the call if the projected cost would push us over 100%.
        let projected = cost_cents_for(decision.primary);
        if self.budget.would_block(projected).await {
            return Err(RouterError::BudgetExceeded(format!(
                "projected cost {projected}¢ would exceed the daily limit"
            )));
        }

        self.queue.begin(request.id.clone(), request.priority).await;

        let result = self.execute_decision(&decision, &request).await;

        self.queue.finish(&request.id).await;

        if let Ok(ref response) = result {
            self.cache.put(cache_key, response.clone()).await;
            let spent = response
                .cost_cents
                .unwrap_or_else(|| cost_cents_for(response.model));
            self.budget
                .record(UsageEntry {
                    timestamp: chrono::Utc::now(),
                    provider: response.model.provider(),
                    model: Some(response.model),
                    task: Some(request.task),
                    cost_cents: spent,
                })
                .await;
        }
        result
    }

    async fn execute_decision(
        &self,
        decision: &RouteDecision,
        request: &AiRequest,
    ) -> Result<AiResponse, RouterError> {
        let mut candidates = vec![decision.primary];
        candidates.extend(decision.fallbacks.iter().copied());

        let mut last_error: Option<ProviderError> = None;
        for model in candidates {
            match self.try_model_with_retry(model, request).await {
                Ok(resp) => return Ok(resp),
                Err(err) if err.is_retriable_on_another_model() => {
                    last_error = Some(err);
                    continue;
                }
                Err(err) => return Err(RouterError::Provider(err)),
            }
        }

        Err(RouterError::AllFallbacksFailed {
            last: last_error.map(|e| e.to_string()),
        })
    }

    async fn try_model_with_retry(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        let provider = model.provider();
        let client = self
            .clients
            .get(&provider)
            .ok_or(ProviderError::NoClient(provider))?;

        let mut attempt = 0u32;
        loop {
            match client.execute(model, request).await {
                Ok(resp) => return Ok(resp),
                Err(err) if attempt + 1 < self.retry.max_attempts && err.is_retriable() => {
                    // Honor provider-supplied Retry-After when present —
                    // otherwise fall back to policy-driven exponential
                    // backoff (debug-review I5).
                    let delay = match &err {
                        ProviderError::RateLimited(d) => *d,
                        _ => self.retry.backoff_for(attempt),
                    };
                    sleep(delay).await;
                    attempt += 1;
                }
                Err(err) => return Err(err),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    struct MockClient {
        provider: Provider,
        calls: AtomicUsize,
        fail_n: usize,
        permanent_fail: bool,
        /// If `Some`, every call fails with this fixed error (clone-on-call).
        /// Used by tests that need to assert non-retriable variants like
        /// `JobAlreadySubmitted` without complicating the `fail_n` counting.
        fixed_error: Option<ProviderError>,
    }

    impl MockClient {
        fn new(provider: Provider, fail_n: usize, permanent_fail: bool) -> Self {
            Self {
                provider,
                calls: AtomicUsize::new(0),
                fail_n,
                permanent_fail,
                fixed_error: None,
            }
        }

        /// Build a client that always returns the given error verbatim. The
        /// `calls` counter still increments so tests can assert call count.
        fn failing_with(provider: Provider, err: ProviderError) -> Self {
            Self {
                provider,
                calls: AtomicUsize::new(0),
                fail_n: 0,
                permanent_fail: false,
                fixed_error: Some(err),
            }
        }
    }

    #[async_trait]
    impl AiClient for MockClient {
        fn provider(&self) -> Provider {
            self.provider
        }
        fn supports(&self, _model: Model) -> bool {
            true
        }
        async fn execute(
            &self,
            model: Model,
            request: &AiRequest,
        ) -> Result<AiResponse, ProviderError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if let Some(err) = &self.fixed_error {
                return Err(err.clone());
            }
            if self.permanent_fail {
                return Err(ProviderError::Permanent("nope".into()));
            }
            if n < self.fail_n {
                return Err(ProviderError::Transient("flaky".into()));
            }
            Ok(AiResponse {
                request_id: request.id.clone(),
                model,
                output: serde_json::Value::Null,
                cost_cents: Some(0),
                cached: false,
            })
        }
        async fn health_check(&self) -> bool {
            true
        }
        async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
            Ok(ProviderUsage::default())
        }
    }

    fn text_request() -> AiRequest {
        AiRequest {
            id: "r1".into(),
            task: TaskKind::TextGeneration,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: "hi".into(),
            payload: serde_json::Value::Null,
        }
    }

    fn zero_retry_policy() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            base: Duration::from_millis(0),
            factor: 1,
            max: Duration::from_millis(0),
        }
    }

    fn router_with(clients: Vec<(Provider, Arc<dyn AiClient>)>) -> AiRouter {
        AiRouter::new(
            Arc::new(DefaultRoutingStrategy),
            clients.into_iter().collect(),
            zero_retry_policy(),
            Arc::new(PriorityQueue::new()),
        )
    }

    #[tokio::test]
    async fn route_calls_the_primary_client_when_it_succeeds() {
        let client = Arc::new(MockClient::new(Provider::Claude, 0, false));
        let router = router_with(vec![(
            Provider::Claude,
            client.clone() as Arc<dyn AiClient>,
        )]);
        let resp = router.route(text_request()).await.unwrap();
        assert_eq!(resp.model, Model::ClaudeSonnet);
        assert_eq!(client.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn route_retries_transient_errors_before_giving_up() {
        let client = Arc::new(MockClient::new(Provider::Claude, 2, false));
        let router = router_with(vec![(
            Provider::Claude,
            client.clone() as Arc<dyn AiClient>,
        )]);
        router.route(text_request()).await.unwrap();
        // 2 fails + 1 success = 3 attempts (matches max_attempts)
        assert_eq!(client.calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn route_falls_back_to_next_model_when_primary_exhausts_retries() {
        // Video task uses fallback chain Kling → Runway → Higgsfield.
        let kling = Arc::new(MockClient::new(Provider::Kling, 99, false));
        let runway = Arc::new(MockClient::new(Provider::Runway, 0, false));
        let router = router_with(vec![
            (Provider::Kling, kling.clone() as Arc<dyn AiClient>),
            (Provider::Runway, runway.clone() as Arc<dyn AiClient>),
        ]);
        let resp = router
            .route(AiRequest {
                id: "v1".into(),
                task: TaskKind::TextToVideo,
                priority: Priority::Normal,
                complexity: Complexity::Medium,
                prompt: "a clip".into(),
                payload: serde_json::Value::Null,
            })
            .await
            .unwrap();
        assert_eq!(resp.model, Model::RunwayGen3);
        // Kling: 3 attempts (all fail). Runway: 1 success.
        assert_eq!(kling.calls.load(Ordering::SeqCst), 3);
        assert_eq!(runway.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn route_returns_error_when_all_fallbacks_exhausted() {
        let kling = Arc::new(MockClient::new(Provider::Kling, 99, false));
        let runway = Arc::new(MockClient::new(Provider::Runway, 99, false));
        let higgs = Arc::new(MockClient::new(Provider::Higgsfield, 99, false));
        let router = router_with(vec![
            (Provider::Kling, kling as Arc<dyn AiClient>),
            (Provider::Runway, runway as Arc<dyn AiClient>),
            (Provider::Higgsfield, higgs as Arc<dyn AiClient>),
        ]);
        let err = router
            .route(AiRequest {
                id: "v2".into(),
                task: TaskKind::TextToVideo,
                priority: Priority::Normal,
                complexity: Complexity::Medium,
                prompt: "a clip".into(),
                payload: serde_json::Value::Null,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, RouterError::AllFallbacksFailed { .. }));
    }

    #[tokio::test]
    async fn route_short_circuits_on_permanent_error() {
        // Permanent errors should NOT retry and NOT fall back.
        let client = Arc::new(MockClient::new(Provider::Claude, 0, true));
        let router = router_with(vec![(
            Provider::Claude,
            client.clone() as Arc<dyn AiClient>,
        )]);
        let err = router.route(text_request()).await.unwrap_err();
        assert!(matches!(
            err,
            RouterError::Provider(ProviderError::Permanent(_))
        ));
        assert_eq!(client.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn route_caches_successful_responses_and_serves_subsequent_calls() {
        let client = Arc::new(MockClient::new(Provider::Claude, 0, false));
        let router = router_with(vec![(
            Provider::Claude,
            client.clone() as Arc<dyn AiClient>,
        )]);

        let first = router.route(text_request()).await.unwrap();
        assert!(!first.cached);

        let second = router.route(text_request()).await.unwrap();
        assert!(second.cached);

        // Client was hit only once — second call came from cache.
        assert_eq!(client.calls.load(Ordering::SeqCst), 1);

        let stats = router.cache().stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.size, 1);
    }

    #[tokio::test]
    async fn route_does_not_cache_when_all_attempts_fail() {
        let client = Arc::new(MockClient::new(Provider::Claude, 99, false));
        let router = router_with(vec![(
            Provider::Claude,
            client.clone() as Arc<dyn AiClient>,
        )]);
        let _ = router.route(text_request()).await;
        assert_eq!(router.cache().len().await, 0);
    }

    #[tokio::test]
    async fn route_blocks_when_budget_ceiling_hit() {
        let client = Arc::new(MockClient::new(Provider::Fal, 0, false));
        let router = router_with(vec![(Provider::Fal, client as Arc<dyn AiClient>)]);
        // Force the daily budget to a ridiculously low ceiling the first
        // projected call will already exceed.
        router
            .budget()
            .set_limits(BudgetLimits {
                daily_cents: Some(1),
                session_cents: None,
            })
            .await;
        // Also seed spent=1 so we're already at 100%.
        router
            .budget()
            .record(UsageEntry {
                timestamp: chrono::Utc::now(),
                provider: Provider::Fal,
                model: None,
                task: None,
                cost_cents: 1,
            })
            .await;

        let err = router
            .route(AiRequest {
                id: "r1".into(),
                task: TaskKind::ImageGeneration,
                priority: Priority::Normal,
                complexity: Complexity::Simple,
                prompt: "blocked".into(),
                payload: serde_json::Value::Null,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, RouterError::BudgetExceeded(_)));
    }

    #[tokio::test]
    async fn route_records_spend_on_success_using_response_cost() {
        let client = Arc::new(MockClient::new(Provider::Fal, 0, false));
        let router = router_with(vec![(Provider::Fal, client as Arc<dyn AiClient>)]);

        let mut req = text_request();
        req.task = TaskKind::ImageGeneration;
        req.complexity = Complexity::Simple;
        // MockClient returns cost_cents = Some(0); budget records 0 and
        // then the fallback cost_cents_for kicks in via `unwrap_or`.
        // The response carries Some(0), so recorded = 0. Ensure the entry
        // is still recorded (so the CSV export sees the request).
        let _ = router.route(req).await.unwrap();
        let entries = router.budget().entries().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].provider, Provider::Fal);
        assert_eq!(entries[0].task, Some(TaskKind::ImageGeneration));
    }

    /// A mock that returns the given sequence of errors (one per call) and
    /// then succeeds on every subsequent call. Used to observe how the
    /// router paces retries through real `tokio::time::sleep`.
    struct SequencedErrorClient {
        provider: Provider,
        errors: std::sync::Mutex<std::collections::VecDeque<ProviderError>>,
        calls: AtomicUsize,
    }

    impl SequencedErrorClient {
        fn new(provider: Provider, errors: Vec<ProviderError>) -> Self {
            Self {
                provider,
                errors: std::sync::Mutex::new(errors.into()),
                calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl AiClient for SequencedErrorClient {
        fn provider(&self) -> Provider {
            self.provider
        }
        fn supports(&self, _model: Model) -> bool {
            true
        }
        async fn execute(
            &self,
            model: Model,
            request: &AiRequest,
        ) -> Result<AiResponse, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let next = self.errors.lock().unwrap().pop_front();
            if let Some(err) = next {
                return Err(err);
            }
            Ok(AiResponse {
                request_id: request.id.clone(),
                model,
                output: serde_json::Value::Null,
                cost_cents: Some(0),
                cached: false,
            })
        }
        async fn health_check(&self) -> bool {
            true
        }
        async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
            Ok(ProviderUsage::default())
        }
    }

    /// Regression for debug-review I5: RateLimited(d) must sleep for `d`, not
    /// for the policy's exponential backoff. With the bug the router would
    /// wait ~200ms (policy default) regardless of the provider's Retry-After;
    /// with the fix we wait ≥50ms (the honored Retry-After) but still well
    /// under the ~200ms policy backoff — so a bracketed assertion proves the
    /// delay came from the provider, not the policy.
    #[tokio::test]
    async fn route_honors_provider_retry_after_over_policy_backoff() {
        let client = Arc::new(SequencedErrorClient::new(
            Provider::Claude,
            vec![ProviderError::RateLimited(Duration::from_millis(50))],
        ));
        // Use the DEFAULT policy (200ms base, 2× factor, 5s max) so the bug
        // and the fix actually differ. If this test ran with a zero policy
        // nothing would be detected.
        let router = AiRouter::new(
            Arc::new(DefaultRoutingStrategy),
            [(Provider::Claude, client.clone() as Arc<dyn AiClient>)]
                .into_iter()
                .collect(),
            RetryPolicy::default_policy(),
            Arc::new(PriorityQueue::new()),
        );

        let start = std::time::Instant::now();
        router.route(text_request()).await.unwrap();
        let elapsed = start.elapsed();

        // Must have slept AT LEAST the honored Retry-After.
        assert!(
            elapsed >= Duration::from_millis(50),
            "expected ≥50ms elapsed (Retry-After honored), got {elapsed:?}"
        );
        // Must NOT have slept the full default-policy backoff (200ms base).
        // Give ourselves generous headroom (150ms) for scheduler jitter and
        // the cost of two client calls on a loaded CI box.
        assert!(
            elapsed < Duration::from_millis(150),
            "expected <150ms elapsed (policy backoff would be ≥200ms), got {elapsed:?}"
        );
        assert_eq!(client.calls.load(Ordering::SeqCst), 2);
    }

    /// Regression for debug-review C1: when a POST-then-poll client surfaces
    /// `JobAlreadySubmitted`, the router MUST NOT retry the same model AND
    /// MUST NOT fall back to a different model. Either action would create a
    /// duplicate billable job server-side. The error must propagate
    /// untouched, with exactly ONE call recorded on the primary client and
    /// ZERO calls on the fallback.
    #[tokio::test]
    async fn route_does_not_retry_or_fall_back_on_job_already_submitted() {
        let kling = Arc::new(MockClient::failing_with(
            Provider::Kling,
            ProviderError::JobAlreadySubmitted("task-abc still pending".into()),
        ));
        let runway = Arc::new(MockClient::new(Provider::Runway, 0, false));
        let router = router_with(vec![
            (Provider::Kling, kling.clone() as Arc<dyn AiClient>),
            (Provider::Runway, runway.clone() as Arc<dyn AiClient>),
        ]);
        let err = router
            .route(AiRequest {
                id: "v-no-retry".into(),
                task: TaskKind::TextToVideo,
                priority: Priority::Normal,
                complexity: Complexity::Medium,
                prompt: "a clip".into(),
                payload: serde_json::Value::Null,
            })
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                RouterError::Provider(ProviderError::JobAlreadySubmitted(_))
            ),
            "expected JobAlreadySubmitted, got {err:?}"
        );
        // CRITICAL: only ONE call on the primary — no in-model retry.
        assert_eq!(kling.calls.load(Ordering::SeqCst), 1);
        // CRITICAL: ZERO calls on the fallback — no cross-model fallback.
        assert_eq!(runway.calls.load(Ordering::SeqCst), 0);
    }
}
