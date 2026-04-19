use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use terryblemachine_lib::ai_router::{
    AiClient, AiRequest, AiResponse, AiRouter, DefaultRoutingStrategy, Model, PriorityQueue,
    Provider, ProviderError, ProviderUsage, RetryPolicy,
};
use terryblemachine_lib::storyboard_generator::{
    ClaudeStoryboardGenerator, StoryboardGenerator, StoryboardInput, StoryboardTemplate,
};

struct StubClaude;

#[async_trait]
impl AiClient for StubClaude {
    fn provider(&self) -> Provider {
        Provider::Claude
    }
    fn supports(&self, m: Model) -> bool {
        matches!(
            m,
            Model::ClaudeHaiku | Model::ClaudeSonnet | Model::ClaudeOpus
        )
    }
    async fn execute(&self, _model: Model, req: &AiRequest) -> Result<AiResponse, ProviderError> {
        let text = json!({
            "summary": "test",
            "template": "commercial",
            "shots": [
                {"index":1,"description":"open","style":"warm","duration_s":5,"camera":"dolly","transition":"fade"},
                {"index":2,"description":"middle","style":"warm","duration_s":5,"camera":"static","transition":"cut"}
            ]
        })
        .to_string();
        Ok(AiResponse {
            request_id: req.id.clone(),
            model: Model::ClaudeSonnet,
            output: json!({ "text": text, "stop_reason": "end_turn" }),
            cost_cents: None,
            cached: false,
        })
    }
    async fn health_check(&self) -> bool {
        true
    }
    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
        Ok(ProviderUsage::default())
    }
}

fn generator() -> ClaudeStoryboardGenerator {
    let mut m: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();
    m.insert(Provider::Claude, Arc::new(StubClaude));
    let router = Arc::new(AiRouter::new(
        Arc::new(DefaultRoutingStrategy),
        m,
        RetryPolicy::default_policy(),
        Arc::new(PriorityQueue::new()),
    ));
    ClaudeStoryboardGenerator::new(router)
}

#[tokio::test]
async fn generates_storyboard_from_text() {
    let g = generator();
    let sb = g
        .generate(StoryboardInput {
            prompt: "coffee shop ad".into(),
            template: StoryboardTemplate::Commercial,
            module: "video".into(),
            model_override: None,
        })
        .await
        .unwrap();
    assert_eq!(sb.shots.len(), 2);
    assert_eq!(sb.shots[0].index, 1);
    assert!(sb.summary.contains("test"));
}

#[tokio::test]
async fn rejects_empty_prompt() {
    let g = generator();
    let err = g
        .generate(StoryboardInput {
            prompt: "   ".into(),
            template: StoryboardTemplate::Commercial,
            module: "video".into(),
            model_override: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        terryblemachine_lib::storyboard_generator::StoryboardError::InvalidInput(_)
    ));
}
