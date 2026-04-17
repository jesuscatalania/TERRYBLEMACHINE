//! Tauri IPC commands for the AI router.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use super::{AiRequest, AiResponse, AiRouter, Provider, ProviderError, QueueStatus, RouterError};

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
}

impl From<RouterError> for RouterIpcError {
    fn from(value: RouterError) -> Self {
        match value {
            RouterError::Provider(err) => provider_to_ipc(err),
            RouterError::AllFallbacksFailed { last } => Self::AllFallbacksFailed(last),
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
