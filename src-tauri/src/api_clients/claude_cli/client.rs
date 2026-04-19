//! ClaudeCliClient — uses the local `claude` CLI as transport instead of HTTP.
//!
//! Spawns `claude -p "<prompt>" --model <slug> --output-format stream-json`
//! via tokio::process. Parses streamed events, maps to AiResponse.
//!
//! Authentication piggy-backs on the user's `claude login` (subscription).
//! No API key is read from our keychain.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::json;

use super::stream_parser::StreamAccumulator;
use crate::ai_router::{
    AiClient, AiRequest, AiResponse, Model, Provider, ProviderError, ProviderUsage,
};

#[derive(Debug, Clone)]
pub struct SpawnResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[async_trait]
pub trait Spawner: Send + Sync {
    async fn spawn(&self, bin: &Path, args: &[String], stdin: Option<String>) -> SpawnResult;
}

/// Real spawner using tokio::process.
pub struct TokioSpawner;

#[async_trait]
impl Spawner for TokioSpawner {
    async fn spawn(&self, bin: &Path, args: &[String], stdin: Option<String>) -> SpawnResult {
        use tokio::io::AsyncWriteExt;
        use tokio::process::Command;
        let mut cmd = Command::new(bin);
        cmd.args(args);
        cmd.kill_on_drop(true);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        if stdin.is_some() {
            cmd.stdin(std::process::Stdio::piped());
        }
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return SpawnResult {
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: format!("spawn failed: {e}"),
                }
            }
        };
        if let (Some(text), Some(mut stdin_handle)) = (stdin, child.stdin.take()) {
            let _ = stdin_handle.write_all(text.as_bytes()).await;
            let _ = stdin_handle.shutdown().await;
        }
        let out = match child.wait_with_output().await {
            Ok(o) => o,
            Err(e) => {
                return SpawnResult {
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: format!("wait failed: {e}"),
                }
            }
        };
        SpawnResult {
            exit_code: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }
    }
}

pub struct ClaudeCliClient {
    binary: PathBuf,
    spawner: Box<dyn Spawner>,
}

impl ClaudeCliClient {
    pub fn new(binary: PathBuf) -> Self {
        Self {
            binary,
            spawner: Box::new(TokioSpawner),
        }
    }

    #[cfg(test)]
    pub fn with_spawner(binary: PathBuf, spawner: Box<dyn Spawner>) -> Self {
        Self { binary, spawner }
    }

    fn model_slug(model: Model) -> Option<&'static str> {
        match model {
            Model::ClaudeOpus => Some("claude-opus-4-7"),
            Model::ClaudeSonnet => Some("claude-sonnet-4-6"),
            Model::ClaudeHaiku => Some("claude-haiku-4-5"),
            _ => None,
        }
    }

    async fn run(
        &self,
        model: Model,
        prompt: &str,
    ) -> Result<super::stream_parser::StreamResult, ProviderError> {
        let slug = Self::model_slug(model).ok_or_else(|| {
            ProviderError::Permanent(format!("ClaudeCliClient: unsupported model {model:?}"))
        })?;
        let args = vec![
            "-p".to_string(),
            prompt.to_string(),
            "--model".to_string(),
            slug.to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
        ];
        let result = self.spawner.spawn(&self.binary, &args, None).await;
        if result.exit_code != 0 {
            return Err(ProviderError::Permanent(format!(
                "claude CLI exit {}: {}",
                result.exit_code,
                result.stderr.trim()
            )));
        }
        let mut acc = StreamAccumulator::new();
        for line in result.stdout.lines() {
            acc.feed_line(line);
        }
        acc.finish().map_err(|err_msg| {
            // Heuristic: auth-flavored messages → Auth variant
            let lower = err_msg.to_lowercase();
            if lower.contains("auth") || lower.contains("token") || lower.contains("login") {
                ProviderError::Auth(err_msg)
            } else {
                ProviderError::Permanent(err_msg)
            }
        })
    }
}

#[async_trait]
impl AiClient for ClaudeCliClient {
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    fn supports(&self, model: Model) -> bool {
        Self::model_slug(model).is_some()
    }

    async fn execute(
        &self,
        model: Model,
        request: &AiRequest,
    ) -> Result<AiResponse, ProviderError> {
        let stream = self.run(model, &request.prompt).await?;
        Ok(AiResponse {
            request_id: request.id.clone(),
            model,
            output: json!({
                "text": stream.text,
                "transport": "cli",
            }),
            cost_cents: stream.cost_usd.map(|usd| (usd * 100.0).round() as u64),
            cached: false,
        })
    }

    async fn health_check(&self) -> bool {
        self.binary.exists()
    }

    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
        // Claude CLI uses the user's subscription — no per-account usage API.
        // Per-request cost is tracked via `result.total_cost_usd` in run().
        Ok(ProviderUsage {
            notes: Some("tracked per-request via stream-json total_cost_usd".into()),
            ..ProviderUsage::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_router::{AiRequest, Complexity, Model, Priority, TaskKind};

    fn req(prompt: &str) -> AiRequest {
        AiRequest {
            id: "t1".into(),
            task: TaskKind::TextGeneration,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: prompt.into(),
            payload: serde_json::Value::Null,
            model_override: None,
        }
    }

    struct MockSpawner {
        canned_stdout: String,
        canned_exit: i32,
    }

    #[async_trait::async_trait]
    impl Spawner for MockSpawner {
        async fn spawn(
            &self,
            _bin: &std::path::Path,
            _args: &[String],
            _stdin: Option<String>,
        ) -> SpawnResult {
            SpawnResult {
                exit_code: self.canned_exit,
                stdout: self.canned_stdout.clone(),
                stderr: String::new(),
            }
        }
    }

    fn opus_response() -> String {
        r#"{"type":"system","subtype":"init","model":"claude-opus-4-7","session_id":"abc"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Generated content here"}]}}
{"type":"result","subtype":"success","total_cost_usd":0.001,"usage":{"input_tokens":10,"output_tokens":4}}
"#
        .to_string()
    }

    #[tokio::test]
    async fn happy_path_text_generation() {
        let spawner = MockSpawner {
            canned_stdout: opus_response(),
            canned_exit: 0,
        };
        let client = ClaudeCliClient::with_spawner(
            std::path::PathBuf::from("/fake/claude"),
            Box::new(spawner),
        );
        let resp = client
            .execute(Model::ClaudeOpus, &req("hello"))
            .await
            .unwrap();
        assert_eq!(resp.model, Model::ClaudeOpus);
        let text = resp.output.get("text").and_then(|t| t.as_str()).unwrap();
        assert_eq!(text, "Generated content here");
    }

    #[tokio::test]
    async fn maps_non_zero_exit_to_permanent() {
        let spawner = MockSpawner {
            canned_stdout: String::new(),
            canned_exit: 1,
        };
        let client = ClaudeCliClient::with_spawner(
            std::path::PathBuf::from("/fake/claude"),
            Box::new(spawner),
        );
        let err = client
            .execute(Model::ClaudeSonnet, &req("hello"))
            .await
            .expect_err("should fail");
        assert!(matches!(err, crate::ai_router::ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn maps_error_subtype_to_provider_error() {
        let spawner = MockSpawner {
            canned_stdout: r#"{"type":"result","subtype":"error_during_execution","error":{"message":"auth: token expired"}}"#.into(),
            canned_exit: 0,
        };
        let client = ClaudeCliClient::with_spawner(
            std::path::PathBuf::from("/fake/claude"),
            Box::new(spawner),
        );
        let err = client
            .execute(Model::ClaudeHaiku, &req("hello"))
            .await
            .expect_err("should fail");
        // Auth-flavored error → Auth variant
        assert!(matches!(err, crate::ai_router::ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn supports_only_claude_models() {
        let spawner = MockSpawner {
            canned_stdout: opus_response(),
            canned_exit: 0,
        };
        let client = ClaudeCliClient::with_spawner(
            std::path::PathBuf::from("/fake/claude"),
            Box::new(spawner),
        );
        assert!(client.supports(Model::ClaudeOpus));
        assert!(client.supports(Model::ClaudeSonnet));
        assert!(client.supports(Model::ClaudeHaiku));
        assert!(!client.supports(Model::FalFluxPro));
    }
}
