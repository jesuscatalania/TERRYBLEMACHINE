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

/// Response shape for the refine round-trip. Same fields as
/// [`LlmResponse`] — kept as a sibling type so a future divergence
/// (e.g. refine-specific metadata) won't force a hack on the generate
/// side.
#[derive(Debug, Deserialize)]
struct RefineLlmResponse {
    #[serde(default)]
    summary: String,
    files: Vec<LlmFile>,
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
            model_override: input.model_override,
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

    async fn refine(
        &self,
        project: GeneratedProject,
        instruction: &str,
    ) -> Result<(GeneratedProject, Vec<String>), CodeGenError> {
        let instr = instruction.trim();
        if instr.is_empty() {
            return Err(CodeGenError::InvalidInput("instruction is empty".into()));
        }
        if project.files.is_empty() {
            return Err(CodeGenError::InvalidInput(
                "no current project to refine".into(),
            ));
        }

        let current_files_blob = project
            .files
            .iter()
            .map(|f| format!("=== FILE: {} ===\n{}\n", f.path.display(), f.content))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "You are refining an existing multi-file project. Below is the CURRENT project state, followed by the user's refinement instruction. Return a STRICT JSON object with this shape:\n\n\
{{\n  \"summary\": \"brief 1-line description of what you changed\",\n  \"files\": [\n    {{ \"path\": \"path/to/file.ext\", \"content\": \"full new file contents\" }}\n  ]\n}}\n\n\
Rules:\n- Include ONLY files you modified. Untouched files MUST be omitted.\n- If you delete a file, include it with `\"content\": \"\"` and we will prune empty files after.\n- If the instruction is unclear or contradicts the existing architecture, make the minimal sensible change and proceed.\n- Do NOT wrap your output in markdown fences. Do NOT add preamble or explanation.\n\n\
CURRENT PROJECT:\n{current_files_blob}\n\n\
USER INSTRUCTION:\n{instr}\n\n\
Respond with the JSON object now."
        );

        let request = AiRequest {
            id: uuid::Uuid::new_v4().to_string(),
            task: TaskKind::TextGeneration,
            priority: Priority::High,
            complexity: Complexity::Complex,
            prompt,
            payload: serde_json::Value::Null,
            model_override: None,
        };

        let response = self
            .client
            .execute(self.model, &request)
            .await
            .map_err(provider_to_gen_err)?;

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

        let parsed = parse_refine_json(raw)?;

        // Merge returned files back into the project. Empty content = delete.
        let mut merged = project.files.clone();
        let mut changed: Vec<String> = Vec::new();
        for f in parsed.files {
            let path_buf: std::path::PathBuf = f.path.clone().into();
            changed.push(f.path);
            if f.content.is_empty() {
                merged.retain(|existing| existing.path != path_buf);
                continue;
            }
            if let Some(slot) = merged.iter_mut().find(|e| e.path == path_buf) {
                slot.content = f.content;
            } else {
                merged.push(GeneratedFile {
                    path: path_buf,
                    content: f.content,
                });
            }
        }

        let summary = if parsed.summary.is_empty() {
            project.summary.clone()
        } else {
            parsed.summary
        };

        Ok((
            GeneratedProject {
                summary,
                files: merged,
                prompt: project.prompt,
            },
            changed,
        ))
    }
}

fn parse_refine_json(raw: &str) -> Result<RefineLlmResponse, CodeGenError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CodeGenError::ParseResponse(
            "Claude returned empty refine response".into(),
        ));
    }

    let candidates = build_parse_candidates(trimmed);
    let mut last_error: Option<serde_json::Error> = None;
    for candidate in &candidates {
        match serde_json::from_str::<RefineLlmResponse>(candidate) {
            Ok(parsed) => return Ok(parsed),
            Err(e) => last_error = Some(e),
        }
    }
    let preview: String = trimmed.chars().take(400).collect();
    let err_msg = match last_error {
        Some(e) => format!("{e}"),
        None => "unknown".into(),
    };
    Err(CodeGenError::ParseResponse(format!(
        "{err_msg}\n--- Claude raw refine output (first 400 chars) ---\n{preview}"
    )))
}

/// Extract the JSON body from whatever Claude returned. Handles:
/// - Pure JSON.
/// - JSON wrapped in ```json fences OR plain ``` fences.
/// - JSON preceded by a one-line preamble ("Here is your project:").
/// - JSON with trailing prose after the closing `}`.
///
/// On failure, includes a preview of Claude's raw response in the error
/// so live-test toasts show WHY the parse failed, not just "expected
/// value at line 1 column 1".
fn parse_llm_json(raw: &str) -> Result<LlmResponse, CodeGenError> {
    let trimmed = raw.trim();

    // Empty responses are a distinct failure mode (claude CLI hit an
    // internal error or ran out of output budget) — call it out.
    if trimmed.is_empty() {
        return Err(CodeGenError::ParseResponse(
            "Claude returned an empty response. Check Settings → Claude transport \
             and confirm `claude` CLI works from your terminal."
                .into(),
        ));
    }

    // Clarifying-question sniff: Claude occasionally ignores the output
    // contract and replies with prose asking the user for more info.
    // Detect by lack of any `{` AND prose shape, and surface a friendlier
    // error so the user knows to enrich the brief (e.g. click Analyze
    // on the reference URL) rather than think the parser itself is broken.
    if !trimmed.contains('{') && looks_like_prose_question(trimmed) {
        let preview: String = trimmed.chars().take(300).collect();
        return Err(CodeGenError::ParseResponse(format!(
            "Claude asked a clarifying question instead of generating the project. \
             This usually means the brief lacks context — e.g. you pasted a Reference \
             URL but didn't click Analyze, so Claude never saw the page contents. \
             Try again with more detail in the brief, or click Analyze first.\n\n\
             Claude said: {preview}"
        )));
    }

    // Try candidates in order of specificity; serialize-only errors on
    // the most promising candidate surface to the user.
    let candidates = build_parse_candidates(trimmed);
    let mut last_error: Option<serde_json::Error> = None;
    for candidate in &candidates {
        match serde_json::from_str::<LlmResponse>(candidate) {
            Ok(parsed) => return Ok(parsed),
            Err(e) => last_error = Some(e),
        }
    }

    let preview: String = trimmed.chars().take(400).collect();
    let err_msg = match last_error {
        Some(e) => format!("{e}"),
        None => "unknown parse failure".to_string(),
    };
    Err(CodeGenError::ParseResponse(format!(
        "{err_msg}\n--- Claude raw output (first 400 chars) ---\n{preview}"
    )))
}

/// Produce ordered candidate strings to try parsing as LlmResponse JSON.
/// More-specific strippings come first so we don't accidentally match a
/// nested `{` in Claude's preamble before the actual payload.
fn build_parse_candidates(trimmed: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    // 1. ```json ... ``` fenced block.
    if let Some(inside) = extract_fenced(trimmed, "```json") {
        out.push(inside.to_string());
    }
    // 2. Plain ``` ... ``` fenced block (Claude sometimes omits `json`).
    if let Some(inside) = extract_fenced(trimmed, "```") {
        out.push(inside.to_string());
    }
    // 3. Balanced-brace extraction: from the first `{` to the matching
    // closing `}`, so trailing prose after the JSON is tolerated.
    if let Some(balanced) = extract_balanced_braces(trimmed) {
        out.push(balanced);
    }
    // 4. Raw trimmed input — last-resort direct parse.
    out.push(trimmed.to_string());
    out
}

/// Extract content between a starting fence (e.g. "```json") and the next
/// closing "```". Returns None if either fence is missing.
fn extract_fenced<'a>(s: &'a str, opener: &str) -> Option<&'a str> {
    let start = s.find(opener).map(|i| i + opener.len())?;
    let rest = &s[start..];
    let end = rest.find("```")?;
    Some(rest[..end].trim())
}

/// Extract substring from the first `{` to its matching `}`, handling
/// nested braces. Returns None if no balanced pair found. Naive — does
/// not understand strings or escapes, so a `{` inside a JSON string
/// value could confuse it. Good enough for Claude's outputs in practice.
fn extract_balanced_braces(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let start = s.find('{')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for (i, b) in bytes.iter().enumerate().skip(start) {
        if escape {
            escape = false;
            continue;
        }
        match *b {
            b'\\' if in_string => escape = true,
            b'"' => in_string = !in_string,
            b'{' if !in_string => depth += 1,
            b'}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(s[start..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Legacy helper retained for the test harness. Same behaviour as the
/// original single-fence-aware extractor.
#[cfg(test)]
fn extract_fenced_json(s: &str) -> Option<&str> {
    extract_fenced(s, "```json")
}

/// Heuristic: does the string look like a human-style clarifying question
/// in English or German rather than generated code? Used to give the user
/// a clearer error than a raw serde failure.
fn looks_like_prose_question(s: &str) -> bool {
    if s.contains('?') {
        return true;
    }
    let lower = s.to_ascii_lowercase();
    // German + English prompts Claude typically uses when stalling.
    const SIGNALS: &[&str] = &[
        "bitte teile",
        "bitte gib",
        "welche seite",
        "welche url",
        "please share",
        "please provide",
        "which page",
        "which url",
        "could you clarify",
        "ich brauche mehr",
        "i need more",
    ];
    SIGNALS.iter().any(|s| lower.contains(s))
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

    #[test]
    fn prose_question_is_detected_german() {
        assert!(looks_like_prose_question(
            "Welche Seite soll ich kopieren? Bitte teile mir die URL mit."
        ));
    }

    #[test]
    fn prose_question_is_detected_english() {
        assert!(looks_like_prose_question(
            "Which page should I clone? Please share a URL."
        ));
    }

    #[test]
    fn valid_json_is_not_a_prose_question() {
        assert!(!looks_like_prose_question(
            r#"{"summary":"x","files":[]}"#
        ));
    }

    #[test]
    fn parse_emits_friendly_error_on_clarifying_question() {
        let raw = "Welche Seite soll ich kopieren? Bitte teile mir die URL mit.";
        let err = parse_llm_json(raw).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("clarifying question"), "got: {msg}");
    }

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
            model_override: None,
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
        assert!(project.prompt.contains("Template default"));
    }

    fn seed_project() -> GeneratedProject {
        GeneratedProject {
            summary: "seed".into(),
            files: vec![
                GeneratedFile {
                    path: std::path::PathBuf::from("index.html"),
                    content: "<h1>Hello</h1>".into(),
                },
                GeneratedFile {
                    path: std::path::PathBuf::from("styles.css"),
                    content: "body { color: blue; }".into(),
                },
            ],
            prompt: "seed prompt".into(),
        }
    }

    #[tokio::test]
    async fn refine_happy_path_merges_changed_files() {
        let client = Arc::new(MockClient {
            text: r#"{"summary":"made it red","files":[{"path":"styles.css","content":"body { color: red; }"}]}"#.into(),
        });
        let gen = ClaudeCodeGenerator::new(client);
        let (project, changed) = gen
            .refine(seed_project(), "make the text red")
            .await
            .expect("refine ok");
        assert_eq!(project.summary, "made it red");
        assert_eq!(project.files.len(), 2);
        let css = project
            .files
            .iter()
            .find(|f| f.path.to_str().unwrap() == "styles.css")
            .unwrap();
        assert_eq!(css.content, "body { color: red; }");
        assert_eq!(changed, vec!["styles.css".to_string()]);
    }

    #[tokio::test]
    async fn refine_adds_new_files_when_claude_returns_a_fresh_path() {
        let client = Arc::new(MockClient {
            text: r##"{"summary":"added readme","files":[{"path":"README.md","content":"# hi"}]}"##
                .into(),
        });
        let gen = ClaudeCodeGenerator::new(client);
        let (project, changed) = gen.refine(seed_project(), "add a readme").await.unwrap();
        assert_eq!(project.files.len(), 3);
        assert!(project
            .files
            .iter()
            .any(|f| f.path.to_str().unwrap() == "README.md"));
        assert_eq!(changed, vec!["README.md".to_string()]);
    }

    #[tokio::test]
    async fn refine_treats_empty_content_as_deletion() {
        let client = Arc::new(MockClient {
            text: r#"{"summary":"rm","files":[{"path":"styles.css","content":""}]}"#.into(),
        });
        let gen = ClaudeCodeGenerator::new(client);
        let (project, changed) = gen.refine(seed_project(), "drop css").await.unwrap();
        assert_eq!(project.files.len(), 1);
        assert!(project
            .files
            .iter()
            .all(|f| f.path.to_str().unwrap() != "styles.css"));
        assert_eq!(changed, vec!["styles.css".to_string()]);
    }

    #[tokio::test]
    async fn refine_preserves_original_summary_when_response_summary_empty() {
        let client = Arc::new(MockClient {
            text: r#"{"summary":"","files":[{"path":"index.html","content":"<h1>Bye</h1>"}]}"#
                .into(),
        });
        let gen = ClaudeCodeGenerator::new(client);
        let (project, _) = gen.refine(seed_project(), "change").await.unwrap();
        assert_eq!(project.summary, "seed");
    }

    #[tokio::test]
    async fn refine_rejects_empty_instruction() {
        let client = Arc::new(MockClient { text: "{}".into() });
        let gen = ClaudeCodeGenerator::new(client);
        let err = gen.refine(seed_project(), "   ").await.unwrap_err();
        assert!(matches!(err, CodeGenError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn refine_rejects_empty_project() {
        let client = Arc::new(MockClient { text: "{}".into() });
        let gen = ClaudeCodeGenerator::new(client);
        let empty = GeneratedProject {
            summary: "".into(),
            files: vec![],
            prompt: "".into(),
        };
        let err = gen.refine(empty, "do stuff").await.unwrap_err();
        assert!(matches!(err, CodeGenError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn refine_malformed_json_yields_parse_error() {
        let client = Arc::new(MockClient {
            text: "no json whatsoever".into(),
        });
        let gen = ClaudeCodeGenerator::new(client);
        let err = gen.refine(seed_project(), "x").await.unwrap_err();
        assert!(matches!(err, CodeGenError::ParseResponse(_)));
    }

    #[tokio::test]
    async fn refine_handles_fenced_json_response() {
        let client = Arc::new(MockClient {
            text: "Sure:\n```json\n{\"summary\":\"ok\",\"files\":[]}\n```".into(),
        });
        let gen = ClaudeCodeGenerator::new(client);
        let (project, changed) = gen.refine(seed_project(), "anything").await.unwrap();
        assert_eq!(project.summary, "ok");
        assert!(changed.is_empty());
    }
}
