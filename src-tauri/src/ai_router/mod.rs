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

pub mod commands;
mod errors;
mod models;
mod queue;
mod router;

pub use errors::{ProviderError, RouterError};
pub use models::{
    AiClient, AiRequest, AiResponse, Complexity, Model, Priority, Provider, TaskKind,
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
}

impl AiRouter {
    pub fn new(
        strategy: Arc<dyn RoutingStrategy>,
        clients: HashMap<Provider, Arc<dyn AiClient>>,
        retry: RetryPolicy,
        queue: Arc<PriorityQueue>,
    ) -> Self {
        Self {
            strategy,
            clients,
            retry,
            queue,
        }
    }

    pub fn queue(&self) -> &Arc<PriorityQueue> {
        &self.queue
    }

    /// Execute a request through the router pipeline.
    ///
    /// 1. The [`RoutingStrategy`] picks a primary model + ordered fallbacks.
    /// 2. Each model is tried with [`RetryPolicy`]-guarded retries.
    /// 3. On success, the first successful [`AiResponse`] is returned.
    /// 4. If every option exhausts its retries, [`RouterError::AllFallbacksFailed`]
    ///    is returned carrying the last provider error.
    pub async fn route(&self, request: AiRequest) -> Result<AiResponse, RouterError> {
        let decision = self.strategy.select(&request);

        self.queue.begin(request.id.clone(), request.priority).await;

        let result = self.execute_decision(&decision, &request).await;

        self.queue.finish(&request.id).await;
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
                    sleep(self.retry.backoff_for(attempt)).await;
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
    }

    impl MockClient {
        fn new(provider: Provider, fail_n: usize, permanent_fail: bool) -> Self {
            Self {
                provider,
                calls: AtomicUsize::new(0),
                fail_n,
                permanent_fail,
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
}
