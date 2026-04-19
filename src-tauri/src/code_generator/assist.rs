//! Inline-edit a code selection via Claude.
//!
//! Given the current files, target path, selected text, and user
//! instruction, Claude returns the replacement text for that selection only.
//! The router handles model selection, caching, retries, and budget gating.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;
use thiserror::Error;

use crate::ai_router::commands::AiRouterState;
use crate::ai_router::{AiRequest, AiRouter, Complexity, Priority, TaskKind};

/// Unique delimiter used to fence the user's selection inside the prompt.
///
/// Chosen so it can never collide with markdown code fences the user might
/// have pasted into their selection (e.g. triple-backtick). If Claude tries
/// to wrap its reply in this delimiter anyway, we don't strip it — we only
/// strip standard markdown fences. The frontend just sees the raw reply.
const SELECTION_DELIM: &str = "<<<<TERRYBLE-SELECTION>>>>";

/// Incoming modify request from the frontend.
///
/// Previously carried a `files: Vec<GeneratedFile>` context field that was
/// shipped over IPC on every keystroke but never consumed by the prompt.
/// Dropped in debug-review Important #3; richer project-context wiring is
/// tracked separately so we don't pay for unused IPC traffic in the meantime.
/// `#[serde(deny_unknown_fields)]` is intentionally NOT set so frontend
/// callers that haven't redeployed yet still succeed (extra `files` field is
/// ignored by the default serde behavior).
#[derive(Debug, Clone, Deserialize)]
pub struct ModifyRequest {
    pub file_path: String,
    pub selection: String,
    pub instruction: String,
}

/// Replacement text that should overwrite the selected range.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ModifyResponse {
    pub replacement: String,
}

/// Typed failures from [`modify_selection`].
///
/// The Tauri boundary still serialises to `Result<_, String>` for the
/// frontend — see [`ModifyError::into_ipc`] — but internally we carry
/// structured variants so callers and tests can pattern-match instead of
/// grepping error strings.
#[derive(Debug, Error)]
pub enum ModifyError {
    #[error("selection is empty")]
    EmptySelection,
    #[error("instruction is empty")]
    EmptyInstruction,
    #[error("router error: {0}")]
    Router(String),
    #[error("empty replacement")]
    EmptyReplacement,
}

impl ModifyError {
    /// Serialise at the Tauri command boundary — the frontend only sees the
    /// human-readable message.
    pub fn into_ipc(self) -> String {
        self.to_string()
    }
}

/// Build the instruction prompt sent through the router.
///
/// The selection is fenced with [`SELECTION_DELIM`] rather than triple
/// backticks so a pasted code block containing ``` cannot close the fence
/// early (prompt injection guard).
pub(crate) fn build_prompt(req: &ModifyRequest) -> String {
    format!(
        "You are editing `{file}` in a React+Tailwind project.\n\n\
Here is the selected snippet, fenced by the markers below:\n\n\
{delim}\n{sel}\n{delim}\n\n\
Apply this change: {instr}\n\n\
Return ONLY the replacement snippet that goes between the markers — \
no markers, no prose, no code fences.",
        file = req.file_path,
        sel = req.selection,
        instr = req.instruction,
        delim = SELECTION_DELIM,
    )
}

/// Strip a single surrounding markdown fence from a multi-line string.
///
/// Triggers only when the first line is an opening fence (``` with an
/// optional language tag) AND the last non-empty line is a closing fence.
/// Everything else is returned as-is so unfenced replies pass through
/// untouched.
fn strip_markdown_fence(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = trimmed.lines().collect();
    if lines.len() < 2 {
        return trimmed.to_string();
    }

    let first = lines[0].trim();
    let is_opening =
        first.starts_with("```") && first[3..].chars().all(|c| c.is_ascii_alphanumeric());
    if !is_opening {
        return trimmed.to_string();
    }

    // Find the last non-empty line and require it to be a bare ``` fence.
    let mut last_idx: Option<usize> = None;
    for (i, line) in lines.iter().enumerate().rev() {
        if !line.trim().is_empty() {
            last_idx = Some(i);
            break;
        }
    }
    let Some(last_i) = last_idx else {
        return trimmed.to_string();
    };
    if last_i == 0 {
        return trimmed.to_string();
    }
    if lines[last_i].trim() != "```" {
        return trimmed.to_string();
    }

    lines[1..last_i].join("\n").trim().to_string()
}

// Extract the text payload from whatever the Claude wrapper returned.
//
// `ClaudeClient::send_messages` wraps the content as
// `{ "text": "...", "stop_reason": "..." }`. As a defensive fallback we
// also support the raw Anthropic `{ "content": [{ "text": "..." }] }` shape
// for adapters that forward the upstream body verbatim.
//
// After picking the text field we strip a surrounding markdown fence so a
// multi-line reply that starts with ```tsx on its own line and ends with
// ``` on its own line comes back as just the body. The previous
// trim_matches('`') approach left the fence in place once newlines sat
// between it and the edges.
fn extract_text(output: &serde_json::Value) -> String {
    if let Some(t) = output.get("text").and_then(|v| v.as_str()) {
        return strip_markdown_fence(t);
    }
    if let Some(t) = output
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|b| b.get("text"))
        .and_then(|t| t.as_str())
    {
        return strip_markdown_fence(t);
    }
    String::new()
}

/// Core impl — takes a router Arc so tests and the Tauri command share logic.
pub async fn modify_selection(
    router: &AiRouter,
    req: ModifyRequest,
) -> Result<ModifyResponse, ModifyError> {
    if req.selection.trim().is_empty() {
        return Err(ModifyError::EmptySelection);
    }
    if req.instruction.trim().is_empty() {
        return Err(ModifyError::EmptyInstruction);
    }

    let prompt = build_prompt(&req);
    let ai_req = AiRequest {
        id: uuid::Uuid::new_v4().to_string(),
        task: TaskKind::TextGeneration,
        priority: Priority::Normal,
        complexity: Complexity::Medium,
        prompt,
        payload: serde_json::Value::Null,
        model_override: None,
    };

    let resp = router
        .route(ai_req)
        .await
        .map_err(|e| ModifyError::Router(e.to_string()))?;
    let replacement = extract_text(&resp.output);
    if replacement.is_empty() {
        return Err(ModifyError::EmptyReplacement);
    }
    Ok(ModifyResponse { replacement })
}

#[tauri::command]
pub async fn modify_code_selection(
    req: ModifyRequest,
    state: State<'_, AiRouterState>,
) -> Result<ModifyResponse, String> {
    let router: Arc<AiRouter> = Arc::clone(&state.0);
    modify_selection(&router, req)
        .await
        .map_err(|e| e.into_ipc())
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
        assert!(matches!(err, ModifyError::EmptySelection));
    }

    #[tokio::test]
    async fn empty_instruction_rejected() {
        let server = MockServer::start().await;
        let router = router_with_claude(server.uri());
        let mut req = sample_request();
        req.instruction = "   ".into();
        let err = modify_selection(&router, req).await.unwrap_err();
        assert!(matches!(err, ModifyError::EmptyInstruction));
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
    fn prompt_uses_unique_selection_delimiter() {
        // A selection that contains triple-backticks must not close the prompt
        // fence early. The outer <<<<TERRYBLE-SELECTION>>>> markers must
        // survive, and the user's backticks must be preserved verbatim inside
        // the body.
        let mut req = sample_request();
        req.selection = "```tsx\n<h1>x</h1>\n```".into();
        let p = build_prompt(&req);
        let delim_hits = p.matches(SELECTION_DELIM).count();
        assert_eq!(
            delim_hits, 2,
            "expected exactly two delimiter markers, got {}: {}",
            delim_hits, p
        );
        assert!(p.contains("```tsx\n<h1>x</h1>\n```"));
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

    #[test]
    fn extract_text_strips_multi_line_tsx_fence() {
        let v = serde_json::json!({
            "text": "```tsx\n<h1 class=\"text-5xl\">Hello</h1>\n```"
        });
        assert_eq!(extract_text(&v), "<h1 class=\"text-5xl\">Hello</h1>");
    }

    #[test]
    fn extract_text_strips_plain_fence() {
        let v = serde_json::json!({
            "text": "```\n<h1>Hi</h1>\n```"
        });
        assert_eq!(extract_text(&v), "<h1>Hi</h1>");
    }

    #[test]
    fn extract_text_leaves_unfenced_text_alone() {
        let v = serde_json::json!({
            "text": "<h1>Hi</h1>\n<p>there</p>"
        });
        assert_eq!(extract_text(&v), "<h1>Hi</h1>\n<p>there</p>");
    }
}
