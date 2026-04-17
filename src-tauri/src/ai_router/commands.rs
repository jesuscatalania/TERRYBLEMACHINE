//! Tauri IPC commands for the AI router.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use super::{
    AiRequest, AiResponse, AiRouter, BudgetLimits, BudgetStatus, CacheStats, Provider,
    ProviderError, QueueStatus, RouterError,
};

pub struct AiRouterState(pub Arc<AiRouter>);

impl AiRouterState {
    pub fn new(router: Arc<AiRouter>) -> Self {
        Self(router)
    }
}

/// IPC error mirror of [`RouterError`] — keeps the frontend API typed.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum RouterIpcError {
    NoClient(Provider),
    Transient(String),
    RateLimited(u64),
    Timeout,
    Auth(String),
    Permanent(String),
    AllFallbacksFailed(Option<String>),
    BudgetExceeded(String),
}

impl From<RouterError> for RouterIpcError {
    fn from(value: RouterError) -> Self {
        match value {
            RouterError::Provider(err) => provider_to_ipc(err),
            RouterError::AllFallbacksFailed { last } => Self::AllFallbacksFailed(last),
            RouterError::BudgetExceeded(msg) => Self::BudgetExceeded(msg),
        }
    }
}

fn provider_to_ipc(err: ProviderError) -> RouterIpcError {
    match err {
        ProviderError::NoClient(p) => RouterIpcError::NoClient(p),
        ProviderError::Transient(msg) => RouterIpcError::Transient(msg),
        ProviderError::RateLimited(d) => RouterIpcError::RateLimited(d.as_millis() as u64),
        ProviderError::Timeout => RouterIpcError::Timeout,
        ProviderError::Auth(msg) => RouterIpcError::Auth(msg),
        ProviderError::Permanent(msg) => RouterIpcError::Permanent(msg),
    }
}

#[tauri::command]
pub async fn route_request(
    request: AiRequest,
    state: State<'_, AiRouterState>,
) -> Result<AiResponse, RouterIpcError> {
    state.0.route(request).await.map_err(Into::into)
}

#[tauri::command]
pub async fn get_queue_status(state: State<'_, AiRouterState>) -> Result<QueueStatus, ()> {
    Ok(state.0.queue().status().await)
}

#[tauri::command]
pub async fn get_cache_stats(state: State<'_, AiRouterState>) -> Result<CacheStats, ()> {
    Ok(state.0.cache().stats().await)
}

#[tauri::command]
pub async fn get_budget_status(state: State<'_, AiRouterState>) -> Result<BudgetStatus, ()> {
    Ok(state.0.budget().status().await)
}

#[tauri::command]
pub async fn set_budget_limit(
    limits: BudgetLimits,
    state: State<'_, AiRouterState>,
) -> Result<BudgetStatus, ()> {
    state.0.budget().set_limits(limits).await;
    Ok(state.0.budget().status().await)
}

#[tauri::command]
pub async fn export_usage(state: State<'_, AiRouterState>) -> Result<String, ()> {
    Ok(state.0.budget().export_csv().await)
}
