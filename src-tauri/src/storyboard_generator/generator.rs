//! ClaudeStoryboardGenerator — routes through AiRouter, parses JSON response.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use super::prompt::build_prompt;
use super::types::{Storyboard, StoryboardError, StoryboardGenerator, StoryboardInput};
use crate::ai_router::{AiRequest, AiRouter, Complexity, Priority, TaskKind};
use crate::taste_engine::TasteEngine;

pub struct ClaudeStoryboardGenerator {
    router: Arc<AiRouter>,
    taste: Option<Arc<TasteEngine>>,
}

impl ClaudeStoryboardGenerator {
    pub fn new(router: Arc<AiRouter>) -> Self {
        Self {
            router,
            taste: None,
        }
    }
    pub fn with_taste_engine(mut self, taste: Arc<TasteEngine>) -> Self {
        self.taste = Some(taste);
        self
    }
}

#[async_trait]
impl StoryboardGenerator for ClaudeStoryboardGenerator {
    async fn generate(&self, input: StoryboardInput) -> Result<Storyboard, StoryboardError> {
        if input.prompt.trim().is_empty() {
            return Err(StoryboardError::InvalidInput("prompt is empty".into()));
        }
        let rules_holder;
        let rules_ref = if let Some(t) = &self.taste {
            let profile = t.profile().await;
            rules_holder = profile.rules;
            Some(&rules_holder)
        } else {
            None
        };
        let prompt = build_prompt(&input, rules_ref);
        let req = AiRequest {
            id: uuid::Uuid::new_v4().to_string(),
            task: TaskKind::TextGeneration,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt,
            payload: json!({}),
            model_override: None,
        };
        let resp = self
            .router
            .route(req)
            .await
            .map_err(|e| StoryboardError::Router(e.to_string()))?;
        let text = resp
            .output
            .get("text")
            .and_then(|v| v.as_str())
            .or_else(|| {
                resp.output
                    .get("content")
                    .and_then(|c| c.as_array())
                    .and_then(|a| a.first())
                    .and_then(|b| b.get("text"))
                    .and_then(|t| t.as_str())
            })
            .unwrap_or("")
            .trim();
        let json_body = strip_fence(text);
        serde_json::from_str::<Storyboard>(json_body)
            .map_err(|e| StoryboardError::Parse(format!("{e}: body was: {json_body}")))
    }
}

fn strip_fence(s: &str) -> &str {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        if let Some(end) = rest.rfind("```") {
            let inner = &rest[..end];
            return inner
                .split_once('\n')
                .map(|(_, body)| body)
                .unwrap_or(inner)
                .trim();
        }
    }
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_fence_handles_json_fence() {
        let input = "```json\n{\"a\":1}\n```";
        assert_eq!(strip_fence(input), "{\"a\":1}");
    }
    #[test]
    fn strip_fence_no_fence_passthrough() {
        assert_eq!(strip_fence("{\"a\":1}"), "{\"a\":1}");
    }
}
