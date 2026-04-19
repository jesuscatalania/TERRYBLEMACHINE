//! Production code generator — dispatches through an [`AiClient`] to Claude
//! Sonnet and parses the JSON response into a [`GeneratedProject`].

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

use crate::ai_router::{AiClient, AiRequest, Complexity, Model, Priority, ProviderError, TaskKind};
use crate::taste_engine::TasteEngine;

use super::prompt::build_prompt;
use super::types::{CodeGenError, CodeGenerator, GeneratedFile, GeneratedProject, GenerationInput};

pub struct ClaudeCodeGenerator {
    client: Arc<dyn AiClient>,
    model: Model,
    taste_engine: Option<Arc<TasteEngine>>,
}

impl ClaudeCodeGenerator {
    pub fn new(client: Arc<dyn AiClient>) -> Self {
        Self {
            client,
            model: Model::ClaudeSonnet,
            taste_engine: None,
        }
    }

    pub fn with_model(mut self, model: Model) -> Self {
        self.model = model;
        self
    }

    pub fn with_taste_engine(mut self, engine: Arc<TasteEngine>) -> Self {
        self.taste_engine = Some(engine);
        self
    }
}

#[derive(Debug, Deserialize)]
struct LlmResponse {
    summary: String,
    files: Vec<LlmFile>,
}

#[derive(Debug, Deserialize)]
struct LlmFile {
    path: String,
    content: String,
}

#[async_trait]
impl CodeGenerator for ClaudeCodeGenerator {
    async fn generate(&self, input: GenerationInput) -> Result<GeneratedProject, CodeGenError> {
        if input.prompt.trim().is_empty() {
            return Err(CodeGenError::InvalidInput("prompt is empty".into()));
        }

        let rules = match &self.taste_engine {
            Some(engine) => Some(engine.profile().await.rules),
            None => None,
        };
        let prompt = build_prompt(&input, rules.as_ref());

        let request = AiRequest {
            id: uuid::Uuid::new_v4().to_string(),
            task: TaskKind::TextGeneration,
            priority: Priority::Normal,
            complexity: Complexity::Complex,
            prompt: prompt.clone(),
            payload: serde_json::Value::Null,
            model_override: None,
        };

        let response = self
            .client
            .execute(self.model, &request)
            .await
            .map_err(provider_to_gen_err)?;

        // Expect the output object to contain `{ "text": "...json..." }`
        // (our Claude client wraps the model's content under `text`).
        let raw = response
            .output
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                CodeGenError::ParseResponse(format!(
                    "response missing `text` field: {:?}",
                    response.output
                ))
            })?;

        let parsed = parse_llm_json(raw)?;

        Ok(GeneratedProject {
            summary: parsed.summary,
            files: parsed
                .files
                .into_iter()
                .map(|f| GeneratedFile {
                    path: f.path.into(),
                    content: f.content,
                })
                .collect(),
            prompt,
        })
    }
}

/// Extract the JSON body from whatever Claude returned. Handles:
/// - Pure JSON.
/// - JSON wrapped in ```json fences.
/// - JSON preceded by a one-line preamble ("Here is your project:").
fn parse_llm_json(raw: &str) -> Result<LlmResponse, CodeGenError> {
    let trimmed = raw.trim();

    // Prefer fenced block if present.
    let candidate = if let Some(inside) = extract_fenced_json(trimmed) {
        inside
    } else {
        // Otherwise find the first "{" and parse from there.
        match trimmed.find('{') {
            Some(idx) => &trimmed[idx..],
            None => trimmed,
        }
    };

    serde_json::from_str::<LlmResponse>(candidate)
        .map_err(|e| CodeGenError::ParseResponse(e.to_string()))
}

fn extract_fenced_json(s: &str) -> Option<&str> {
    let start = s.find("```json").map(|i| i + "```json".len())?;
    let rest = &s[start..];
    let end = rest.find("```")?;
    Some(rest[..end].trim())
}

fn provider_to_gen_err(err: ProviderError) -> CodeGenError {
    CodeGenError::Provider(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_router::{AiResponse, Provider, ProviderUsage};
    use crate::code_generator::templates::Template;
    use async_trait::async_trait;
    use serde_json::json;

    struct MockClient {
        text: String,
    }

    #[async_trait]
    impl AiClient for MockClient {
        fn provider(&self) -> Provider {
            Provider::Claude
        }
        fn supports(&self, _model: Model) -> bool {
            true
        }
        async fn execute(
            &self,
            model: Model,
            request: &AiRequest,
        ) -> Result<AiResponse, ProviderError> {
            Ok(AiResponse {
                request_id: request.id.clone(),
                model,
                output: json!({ "text": self.text }),
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

    fn input(prompt: &str) -> GenerationInput {
        GenerationInput {
            prompt: prompt.into(),
            template: Template::LandingPage,
            reference: None,
            image_path: None,
            module: "website".into(),
        }
    }

    #[tokio::test]
    async fn parses_pure_json_response() {
        let client = Arc::new(MockClient {
            text: r#"{"summary":"ok","files":[{"path":"index.html","content":"<h1>Hi</h1>"}]}"#
                .into(),
        });
        let gen = ClaudeCodeGenerator::new(client);
        let project = gen.generate(input("hi")).await.unwrap();
        assert_eq!(project.summary, "ok");
        assert_eq!(project.files.len(), 1);
        assert_eq!(project.files[0].path.to_str().unwrap(), "index.html");
    }

    #[tokio::test]
    async fn parses_fenced_json_response() {
        let client = Arc::new(MockClient {
            text: "Sure, here you go:\n\n```json\n{\"summary\":\"x\",\"files\":[]}\n```\nEnjoy!"
                .into(),
        });
        let gen = ClaudeCodeGenerator::new(client);
        let project = gen.generate(input("go")).await.unwrap();
        assert_eq!(project.summary, "x");
        assert!(project.files.is_empty());
    }

    #[tokio::test]
    async fn strips_prose_before_json() {
        let client = Arc::new(MockClient {
            text: "Here's your site:\n{\"summary\":\"y\",\"files\":[]}".into(),
        });
        let gen = ClaudeCodeGenerator::new(client);
        let project = gen.generate(input("go")).await.unwrap();
        assert_eq!(project.summary, "y");
    }

    #[tokio::test]
    async fn empty_prompt_is_invalid_input() {
        let client = Arc::new(MockClient { text: "{}".into() });
        let gen = ClaudeCodeGenerator::new(client);
        let err = gen.generate(input("   ")).await.unwrap_err();
        assert!(matches!(err, CodeGenError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn malformed_json_yields_parse_error() {
        let client = Arc::new(MockClient {
            text: "no json here".into(),
        });
        let gen = ClaudeCodeGenerator::new(client);
        let err = gen.generate(input("x")).await.unwrap_err();
        assert!(matches!(err, CodeGenError::ParseResponse(_)));
    }

    #[tokio::test]
    async fn prompt_is_captured_in_result() {
        let client = Arc::new(MockClient {
            text: r#"{"summary":"z","files":[]}"#.into(),
        });
        let gen = ClaudeCodeGenerator::new(client);
        let project = gen.generate(input("my prompt")).await.unwrap();
        assert!(project.prompt.contains("my prompt"));
        assert!(project.prompt.contains("Template:"));
    }
}
