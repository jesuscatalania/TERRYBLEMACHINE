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

const OUTPUT_CONTRACT: &str = "HARD OUTPUT CONTRACT — read before responding:\n- Do NOT say 'I will', 'I need to', 'Let me', 'Here is', or any meta-commentary.\n- Do NOT mention or invoke any skill, tool, or internal process.\n- Do NOT introduce, explain, preface, or wrap your output.\n- Do NOT use quotes, markdown, bullet points, or headers.\n- Output is EXCLUSIVELY the rewritten prompt, as plain prose, on one or more lines.\n- If you cannot rewrite, respond with the original prompt verbatim — still no commentary.\n- The FIRST character of your response must be the FIRST character of the rewritten prompt.";

const META_VISUAL: &str = "You are an expert prompt engineer for AI image and video generation. Rewrite the user's prompt to be more specific, visually rich, and unambiguous, while preserving their intent. Keep the same length scale as the input.";
const META_CODE: &str = "You are an expert prompt engineer for AI code generation. Rewrite the user's prompt to be more specific about expected behavior, edge cases, and constraints, while preserving intent.";
const META_LOGO: &str = "You are an expert prompt engineer for AI logo / typography generation. Rewrite the user's prompt to specify style cues (modern, brutalist, hand-drawn, etc.), color hints, and the exact text to render.";
const META_GENERIC: &str = "Rewrite the user's prompt to be more specific and unambiguous, preserving intent.";

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
    let combined = format!(
        "{meta}\n\n{OUTPUT_CONTRACT}\n\nUSER PROMPT:\n{}\n\nREWRITTEN PROMPT (plain prose, no preamble):",
        input.prompt
    );
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
    let raw = response
        .output
        .get("text")
        .and_then(|t| t.as_str())
        .ok_or_else(|| RouterIpcError::Permanent("optimize: no text in response".to_string()))?;
    Ok(strip_preamble(raw))
}

/// Defensive filter: models occasionally ignore the no-preamble contract and
/// prefix their rewrite with commentary like "I need to invoke the
/// prompt-engineer skill …" or "I'll rewrite this as follows: …". Strip
/// any leading line that looks like meta-commentary, then trim.
fn strip_preamble(raw: &str) -> String {
    const PREAMBLE_MARKERS: &[&str] = &[
        "i need to ",
        "i'll ",
        "i will ",
        "i'm going to ",
        "let me ",
        "here's ",
        "here is ",
        "sure, ",
        "sure! ",
        "okay, ",
        "ok, ",
        "certainly, ",
        "of course, ",
        "i should ",
        "let's ",
    ];

    let mut remaining = raw.trim();

    // Peel off up to three preamble paragraphs; each paragraph is one or
    // more lines separated by a blank line. Three is a safety cap so a
    // legitimate rewrite starting with "I'll" isn't pathologically stripped
    // to nothing.
    for _ in 0..3 {
        let lower = remaining.trim_start().to_ascii_lowercase();
        let has_marker = PREAMBLE_MARKERS.iter().any(|m| lower.starts_with(m));
        if !has_marker {
            break;
        }
        // Find the next blank-line boundary; drop everything up to and
        // including it. If no blank line exists, drop the first sentence-
        // ish line (up to the next newline or period+space).
        let after = remaining.trim_start();
        if let Some(idx) = after.find("\n\n") {
            remaining = &after[idx + 2..];
        } else if let Some(idx) = after.find('\n') {
            remaining = &after[idx + 1..];
        } else if let Some(idx) = after.find(". ") {
            remaining = &after[idx + 2..];
        } else {
            // Single line, no obvious boundary; keep as-is rather than
            // over-strip.
            break;
        }
    }

    remaining.trim().to_string()
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

    #[test]
    fn strip_preamble_removes_skill_invocation_paragraph() {
        let raw = "I need to invoke the prompt-engineer skill for this task since I'm being asked to optimize a prompt for video generation.\n\nZwei muskulöse Katzen im Kampf.";
        assert_eq!(strip_preamble(raw), "Zwei muskulöse Katzen im Kampf.");
    }

    #[test]
    fn strip_preamble_removes_here_is_prefix() {
        let raw = "Here's the rewritten prompt:\n\nA serene sunset over Berlin rooftops.";
        assert_eq!(strip_preamble(raw), "A serene sunset over Berlin rooftops.");
    }

    #[test]
    fn strip_preamble_leaves_clean_output_unchanged() {
        let clean = "A serene sunset over Berlin rooftops, golden hour, 35mm film grain.";
        assert_eq!(strip_preamble(clean), clean);
    }

    #[test]
    fn strip_preamble_handles_german_clean_output_unchanged() {
        let clean = "Zwei Katzen kämpfen in einer Halle, dramatische Beleuchtung, fotorealistisch.";
        assert_eq!(strip_preamble(clean), clean);
    }

    #[test]
    fn strip_preamble_handles_single_line_preamble_with_period() {
        let raw = "Let me rewrite this for you. A dog running on a beach at sunrise.";
        assert_eq!(strip_preamble(raw), "A dog running on a beach at sunrise.");
    }

    #[test]
    fn strip_preamble_peels_multiple_preamble_paragraphs() {
        let raw = "I'll help you with this.\n\nLet me think about the best approach.\n\nA glowing mushroom forest.";
        assert_eq!(strip_preamble(raw), "A glowing mushroom forest.");
    }

    #[test]
    fn strip_preamble_stops_at_three_paragraphs_to_avoid_empty_result() {
        let raw = "I'll do this.\n\nLet me proceed.\n\nI need to think.\n\nI will write something.\n\nActual content.";
        let out = strip_preamble(raw);
        // Should have peeled at most 3; the result still has one I-line + real content.
        assert!(out.contains("Actual content"));
    }
}
