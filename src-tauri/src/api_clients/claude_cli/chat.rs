//! `chat_send_message` Tauri command. Streams response text back to the
//! frontend via `app.emit("chat:stream-chunk", payload)` so the Chat UI can
//! render token-by-token.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use super::client::ClaudeCliClient;
use super::discovery::detect_claude_binary;
use crate::ai_router::{AiRequest, Complexity, Priority, TaskKind};

#[derive(Debug, Deserialize)]
pub struct ChatSendInput {
    pub messages: Vec<ChatMessageInput>,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessageInput {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChunkEvent {
    pub message_id: String,
    pub chunk: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoneEvent {
    pub message_id: String,
    pub error: Option<String>,
}

/// Send a chat message; streams chunks back via Tauri events.
/// Returns the assistant message_id so the frontend can route chunks.
#[tauri::command]
pub async fn chat_send_message(
    input: ChatSendInput,
    app: AppHandle,
    message_id: String,
) -> Result<(), String> {
    // Build a single concatenated prompt (Claude CLI doesn't accept role-tagged
    // multi-message input via -p; we represent history as a transcript).
    let mut transcript = String::new();
    for m in &input.messages {
        transcript.push_str(&format!("{}: {}\n", m.role, m.content));
    }
    transcript.push_str("assistant: ");

    let bin = detect_claude_binary().ok_or_else(|| {
        "claude CLI not found — install: brew install anthropic/claude-code/claude".to_string()
    })?;
    let client = ClaudeCliClient::new(bin);

    // For chat, we use ClaudeSonnet by default (good balance of cost / quality).
    let request = AiRequest {
        id: message_id.clone(),
        task: TaskKind::TextGeneration,
        priority: Priority::Normal,
        complexity: Complexity::Medium,
        prompt: transcript,
        payload: serde_json::Value::Null,
        model_override: None,
    };

    // Use the existing AiClient::execute (which buffers). Streaming via events
    // is left for a follow-up; for now we emit a single chunk with the full
    // result then a done event. This keeps Sub-Project B unblocked.
    use crate::ai_router::AiClient;
    match client
        .execute(crate::ai_router::Model::ClaudeSonnet, &request)
        .await
    {
        Ok(resp) => {
            let text = resp
                .output
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            let _ = app.emit(
                "chat:stream-chunk",
                ChunkEvent {
                    message_id: message_id.clone(),
                    chunk: text,
                },
            );
            let _ = app.emit(
                "chat:stream-done",
                DoneEvent {
                    message_id,
                    error: None,
                },
            );
            Ok(())
        }
        Err(e) => {
            let _ = app.emit(
                "chat:stream-done",
                DoneEvent {
                    message_id,
                    error: Some(e.to_string()),
                },
            );
            Err(e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_send_input_deserializes() {
        let json = r#"{"messages":[{"role":"user","content":"hi"},{"role":"assistant","content":"hello"}]}"#;
        let parsed: ChatSendInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.messages.len(), 2);
        assert_eq!(parsed.messages[0].role, "user");
        assert_eq!(parsed.messages[1].content, "hello");
    }

    #[test]
    fn chunk_event_serializes_with_snake_case_fields() {
        let evt = ChunkEvent {
            message_id: "abc".into(),
            chunk: "world".into(),
        };
        let out = serde_json::to_string(&evt).unwrap();
        assert!(out.contains("\"message_id\":\"abc\""));
        assert!(out.contains("\"chunk\":\"world\""));
    }

    #[test]
    fn done_event_serializes_error_field() {
        let evt = DoneEvent {
            message_id: "x".into(),
            error: Some("boom".into()),
        };
        let out = serde_json::to_string(&evt).unwrap();
        assert!(out.contains("\"message_id\":\"x\""));
        assert!(out.contains("\"error\":\"boom\""));
    }
}
