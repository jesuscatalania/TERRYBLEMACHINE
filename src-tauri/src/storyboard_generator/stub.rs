//! Deterministic stub storyboard generator (for tests and offline dev).

use async_trait::async_trait;

use super::types::{Shot, Storyboard, StoryboardError, StoryboardGenerator, StoryboardInput};

#[derive(Default)]
pub struct StubStoryboardGenerator;

impl StubStoryboardGenerator {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl StoryboardGenerator for StubStoryboardGenerator {
    async fn generate(&self, input: StoryboardInput) -> Result<Storyboard, StoryboardError> {
        if input.prompt.trim().is_empty() {
            return Err(StoryboardError::InvalidInput("prompt empty".into()));
        }
        Ok(Storyboard {
            summary: format!("Stub board for: {}", input.prompt.trim()),
            template: format!("{:?}", input.template).to_lowercase(),
            shots: (1..=5)
                .map(|i| Shot {
                    index: i,
                    description: format!("Stub shot {i} — {}", input.prompt.trim()),
                    style: "neutral, bright".into(),
                    duration_s: 4.0,
                    camera: "static wide".into(),
                    transition: if i == 5 { "cut".into() } else { "fade".into() },
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storyboard_generator::types::StoryboardTemplate;

    #[tokio::test]
    async fn stub_generates_five_shots() {
        let g = StubStoryboardGenerator::new();
        let sb = g
            .generate(StoryboardInput {
                prompt: "coffee".into(),
                template: StoryboardTemplate::Commercial,
                module: "video".into(),
            })
            .await
            .unwrap();
        assert_eq!(sb.shots.len(), 5);
        assert_eq!(sb.shots[0].index, 1);
        assert!(sb.summary.contains("coffee"));
    }

    #[tokio::test]
    async fn stub_rejects_empty_prompt() {
        let g = StubStoryboardGenerator::new();
        let err = g
            .generate(StoryboardInput {
                prompt: "  ".into(),
                template: StoryboardTemplate::Commercial,
                module: "video".into(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, StoryboardError::InvalidInput(_)));
    }
}
