//! Inline-edit a code selection via Claude.
//!
//! Given the current files, target path, selected text, and user
//! instruction, Claude returns the replacement text for that selection only.
//! The router handles model selection, caching, retries, and budget gating.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::ai_router::commands::AiRouterState;
use crate::ai_router::{AiRequest, AiRouter, Complexity, Priority, TaskKind};
use crate::code_generator::types::GeneratedFile;

/// Incoming modify request from the frontend.
///
/// `files` is accepted as context so the prompt can reference the rest of
/// the project if needed (currently we only send the selection, but the
/// payload is reserved for richer context in follow-up work).
#[derive(Debug, Clone, Deserialize)]
pub struct ModifyRequest {
    #[serde(default)]
    pub files: Vec<GeneratedFile>,
    pub file_path: String,
    pub selection: String,
    pub instruction: String,
}

/// Replacement text that should overwrite the selected range.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ModifyResponse {
    pub replacement: String,
}

/// Build the instruction prompt sent through the router.
pub(crate) fn build_prompt(req: &ModifyRequest) -> String {
    format!(
        "You are editing `{file}` in a React + Tailwind project. \
Here is the selected snippet:\n\n```\n{sel}\n```\n\n\
Apply this change: {instr}\n\n\
Return ONLY the replacement snippet, no prose, no code fences, no leading/trailing blank lines.",
        file = req.file_path,
        sel = req.selection,
        instr = req.instruction,
    )
}

/// Extract the text payload from whatever the Claude wrapper returned.
///
/// `ClaudeClient::send_messages` wraps the content as
/// `{ "text": "...", "stop_reason": "..." }`. As a defensive fallback we
/// also support the raw Anthropic `{ "content": [{ "text": "..." }] }` shape
/// for adapters that forward the upstream body verbatim.
fn extract_text(output: &serde_json::Value) -> String {
    if let Some(t) = output.get("text").and_then(|v| v.as_str()) {
        return t.trim().trim_matches('`').trim().to_string();
    }
    if let Some(t) = output
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|b| b.get("text"))
        .and_then(|t| t.as_str())
    {
        return t.trim().trim_matches('`').trim().to_string();
    }
    String::new()
}

/// Core impl — takes a router Arc so tests and the Tauri command share logic.
pub async fn modify_selection(
    router: &AiRouter,
    req: ModifyRequest,
) -> Result<ModifyResponse, String> {
    if req.selection.trim().is_empty() {
        return Err("selection is empty".into());
    }
    if req.instruction.trim().is_empty() {
        return Err("instruction is empty".into());
    }

    let prompt = build_prompt(&req);
    let ai_req = AiRequest {
        id: uuid::Uuid::new_v4().to_string(),
        task: TaskKind::TextGeneration,
        priority: Priority::Normal,
        complexity: Complexity::Medium,
        prompt,
        payload: serde_json::Value::Null,
    };

    let resp = router.route(ai_req).await.map_err(|e| e.to_string())?;
    Ok(ModifyResponse {
        replacement: extract_text(&resp.output),
    })
}

#[tauri::command]
pub async fn modify_code_selection(
    req: ModifyRequest,
    state: State<'_, AiRouterState>,
) -> Result<ModifyResponse, String> {
    let router: Arc<AiRouter> = Arc::clone(&state.0);
    modify_selection(&router, req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_router::{
        AiClient, DefaultRoutingStrategy, PriorityQueue, Provider, RetryPolicy,
    };
    use crate::api_clients::claude::{ClaudeClient, KEYCHAIN_SERVICE};
    use crate::keychain::{InMemoryStore, KeyStore};
    use std::collections::HashMap;
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn router_with_claude(base_url: String) -> Arc<AiRouter> {
        let store = InMemoryStore::new();
        store.store(KEYCHAIN_SERVICE, "sk-test").unwrap();
        let key_store: Arc<dyn KeyStore> = Arc::new(store);
        let claude = Arc::new(ClaudeClient::for_test(key_store, base_url));
        let mut clients: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();
        clients.insert(Provider::Claude, claude);
        let retry = RetryPolicy {
            max_attempts: 1,
            base: Duration::from_millis(0),
            factor: 1,
            max: Duration::from_millis(0),
        };
        Arc::new(AiRouter::new(
            Arc::new(DefaultRoutingStrategy),
            clients,
            retry,
            Arc::new(PriorityQueue::new()),
        ))
    }

    fn sample_request() -> ModifyRequest {
        ModifyRequest {
            files: vec![GeneratedFile {
                path: "index.html".into(),
                content: "<h1>Old</h1>".into(),
            }],
            file_path: "index.html".into(),
            selection: "<h1>Old</h1>".into(),
            instruction: "make the headline larger and bold".into(),
        }
    }

    #[tokio::test]
    async fn happy_path_returns_claude_replacement() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": [{ "type": "text", "text": "<h1 class=\"text-5xl font-bold\">Old</h1>" }],
                "stop_reason": "end_turn"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let router = router_with_claude(server.uri());
        let resp = modify_selection(&router, sample_request()).await.unwrap();
        assert_eq!(
            resp.replacement,
            "<h1 class=\"text-5xl font-bold\">Old</h1>"
        );
    }

    #[tokio::test]
    async fn empty_selection_rejected() {
        let server = MockServer::start().await;
        let router = router_with_claude(server.uri());
        let mut req = sample_request();
        req.selection = "   ".into();
        let err = modify_selection(&router, req).await.unwrap_err();
        assert!(err.contains("selection"));
    }

    #[tokio::test]
    async fn empty_instruction_rejected() {
        let server = MockServer::start().await;
        let router = router_with_claude(server.uri());
        let mut req = sample_request();
        req.instruction = "   ".into();
        let err = modify_selection(&router, req).await.unwrap_err();
        assert!(err.contains("instruction"));
    }

    #[tokio::test]
    async fn prompt_embeds_file_selection_and_instruction() {
        // Spot-check the constructed prompt so future refactors don't silently
        // drop the file path / selection / instruction context.
        let req = sample_request();
        let p = build_prompt(&req);
        assert!(p.contains("index.html"));
        assert!(p.contains("<h1>Old</h1>"));
        assert!(p.contains("make the headline larger"));
        assert!(p.contains("Return ONLY"));
    }

    #[test]
    fn extract_text_prefers_wrapped_text_field() {
        let v = serde_json::json!({ "text": "hello", "stop_reason": "end_turn" });
        assert_eq!(extract_text(&v), "hello");
    }

    #[test]
    fn extract_text_falls_back_to_content_array() {
        let v = serde_json::json!({ "content": [{ "text": "hi there" }] });
        assert_eq!(extract_text(&v), "hi there");
    }

    #[test]
    fn extract_text_returns_empty_when_missing() {
        let v = serde_json::json!({ "stop_reason": "end_turn" });
        assert_eq!(extract_text(&v), "");
    }
}
