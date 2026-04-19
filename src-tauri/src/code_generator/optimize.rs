//! `optimize_prompt` Tauri command — rewrites a user prompt for clarity +
//! visual richness using Claude. Wraps the user's text with a meta-prompt
//! tuned to the TaskKind (visual prompts get a different bias than code).

use serde::Deserialize;
use tauri::State;

use crate::ai_router::commands::{AiRouterState, RouterIpcError};
use crate::ai_router::{AiRequest, Complexity, Priority, TaskKind};

#[derive(Debug, Deserialize)]
pub struct OptimizeInput {
    pub prompt: String,
    pub task: String,
}

const META_VISUAL: &str = "You are an expert prompt engineer for AI image and video generation. Rewrite the user's prompt to be more specific, visually rich, and unambiguous, while preserving their intent. Output ONLY the rewritten prompt — no preamble, no explanation, no quotes. Keep the same length scale as the input.";
const META_CODE: &str = "You are an expert prompt engineer for AI code generation. Rewrite the user's prompt to be more specific about expected behavior, edge cases, and constraints, while preserving intent. Output ONLY the rewritten prompt.";
const META_LOGO: &str = "You are an expert prompt engineer for AI logo / typography generation. Rewrite the user's prompt to specify style cues (modern, brutalist, hand-drawn, etc.), color hints, and the exact text to render. Output ONLY the rewritten prompt.";
const META_GENERIC: &str = "Rewrite the user's prompt to be more specific and unambiguous, preserving intent. Output ONLY the rewritten prompt.";

fn meta_for(task: &str) -> &'static str {
    match task {
        "ImageGeneration" | "ImageEdit" | "Inpaint" | "TextToVideo" | "ImageToVideo" => META_VISUAL,
        "TextGeneration" => META_CODE,
        "Logo" => META_LOGO,
        _ => META_GENERIC,
    }
}

#[tauri::command]
pub async fn optimize_prompt(
    input: OptimizeInput,
    router: State<'_, AiRouterState>,
) -> Result<String, RouterIpcError> {
    let meta = meta_for(&input.task);
    let combined = format!("{meta}\n\nUSER PROMPT:\n{}\n\nREWRITTEN:", input.prompt);
    let request = AiRequest {
        id: uuid::Uuid::new_v4().to_string(),
        task: TaskKind::TextGeneration,
        priority: Priority::High,
        complexity: Complexity::Simple,
        prompt: combined,
        payload: serde_json::Value::Null,
        model_override: None,
    };
    let response = router.0.route(request).await?;
    let text = response
        .output
        .get("text")
        .and_then(|t| t.as_str())
        .ok_or_else(|| RouterIpcError::Permanent("optimize: no text in response".to_string()))?
        .trim()
        .to_string();
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_for_visual_tasks() {
        assert_eq!(meta_for("ImageGeneration"), META_VISUAL);
        assert_eq!(meta_for("TextToVideo"), META_VISUAL);
        assert_eq!(meta_for("Inpaint"), META_VISUAL);
    }

    #[test]
    fn meta_for_code_tasks() {
        assert_eq!(meta_for("TextGeneration"), META_CODE);
    }

    #[test]
    fn meta_for_logo() {
        assert_eq!(meta_for("Logo"), META_LOGO);
    }

    #[test]
    fn meta_for_unknown_falls_back_to_generic() {
        assert_eq!(meta_for("Spaghetti"), META_GENERIC);
    }
}
