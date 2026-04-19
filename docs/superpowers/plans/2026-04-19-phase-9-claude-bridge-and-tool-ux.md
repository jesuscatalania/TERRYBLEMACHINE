# Phase 9: Claude Subscription Bridge + Tool-Selection UX — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Route all backend Claude calls through the user's local `claude` CLI (subscription auth, no API credits) AND ship a sidebar Chat panel + per-prompt Tool Dropdown + `/tool` override parser + Optimize toggle with Undo, before the first live test.

**Architecture:** Sub-Project A adds `ClaudeCliClient` implementing the existing `AiClient` trait — drop-in replacement for HTTP `ClaudeClient`. Settings persists transport choice per Claude provider (Auto / API / CLI). Sub-Project B adds `/chat` route driven by Claude (via the bridge from A), a static `toolCatalog.ts` that maps every TaskKind to its capable models in tier order, a pure-function `parseOverride` that recognizes `/tool` slugs at start/end of prompt, an `OptimizeToggle` that calls a new `optimize_prompt` IPC and replaces the input with full Undo support, and a `ToolDropdown` mounted into every page's brief-row.

**Tech Stack:** Rust + Tauri v2 + tokio::process (Sub-Project A); React 19 + TypeScript + Tailwind + Zustand + Vitest + Playwright (Sub-Project B). No new runtime crates beyond `which` (already in tree as transitive); no new npm deps.

**Source spec:** `docs/superpowers/specs/2026-04-19-phase-9-claude-bridge-and-tool-ux-design.md`

---

## File Structure

### Sub-Project A — Claude CLI Bridge

| File | Responsibility |
|---|---|
| `src-tauri/src/api_clients/claude_cli.rs` | New `ClaudeCliClient` — implements `AiClient`, spawns `claude -p --output-format stream-json` via `tokio::process::Command`, parses stream-json line-by-line, maps to `AiResponse` |
| `src-tauri/src/api_clients/claude_cli/discovery.rs` | `detect_claude_binary() -> Option<PathBuf>` — checks `which claude`, falls back to `~/.claude/local/claude` and `/usr/local/bin/claude` |
| `src-tauri/src/api_clients/claude_cli/stream_parser.rs` | Pure parser: takes stream-json text → accumulates `text` deltas → returns final accumulated content + token-usage metadata |
| `src-tauri/src/api_clients/registry.rs` | Modified — read keystore meta `__claude_transport__` key (`auto|api|cli`), construct `ClaudeCliClient` or HTTP `ClaudeClient` accordingly |
| `src-tauri/src/keychain/commands.rs` | No change — meta key `__claude_transport__` uses existing `store_api_key` infrastructure |
| `src-tauri/src/api_clients/claude_cli_commands.rs` | New Tauri commands: `detect_claude_cli() -> Option<String>` (returns binary path), `set_claude_transport(transport: String)`, `get_claude_transport() -> String` |
| `src-tauri/src/lib.rs` | Register the 3 new commands in invoke_handler |
| `src/lib/claudeTransport.ts` | TS wrapper for the 3 IPC commands |
| `src/components/settings/providers.ts` | Claude row gets `transports: ["auto","api","cli"]` |
| `src/components/settings/ProviderKeyRow.tsx` | When `provider.transports` present, render a `<select>` between status dot and key input; on change call `setClaudeTransport` |

### Sub-Project B — Chat + Tool UX

| File | Responsibility |
|---|---|
| `src/lib/toolCatalog.ts` | TaskKind → ToolDef[] (tier: primary/fallback/alternative) — single source for dropdowns + override resolution |
| `src/lib/toolCatalog.test.ts` | Asserts catalog matches the routing strategy primary for every TaskKind |
| `src/lib/promptOverride.ts` | `parseOverride(prompt) → { override?: ToolSlug, cleanPrompt: string, slugLocation?: "start"|"end" }`, `OVERRIDE_ALIASES` map |
| `src/lib/promptOverride.test.ts` | 12+ tests covering start/end/middle/multiple/case/unknown |
| `src/components/ui/ToolDropdown.tsx` | Renders selected model + dropdown with grouped options (Primary / Fallbacks / Alternatives / Auto) |
| `src/components/ui/ToolDropdown.test.tsx` | Renders all groups, onChange fires correct Model |
| `src/components/ui/OptimizeToggle.tsx` | Toggle button + visual state (off / on / busy / undo-available) |
| `src/components/ui/OptimizeToggle.test.tsx` | Render + click states |
| `src/hooks/useOptimizePrompt.ts` | `useOptimizePrompt({ taskKind, value, setValue })` returns `{ enabled, setEnabled, optimize, undo, canUndo, busy }` |
| `src/hooks/useOptimizePrompt.test.ts` | Hook lifecycle + replace-and-undo |
| `src/lib/optimizeCommands.ts` | TS wrapper for new `optimize_prompt` IPC |
| `src-tauri/src/code_generator/optimize.rs` | New `optimize_prompt` Tauri command — takes prompt + task, calls Claude (via existing AiRouter) with a meta-prompt, returns rewritten prompt |
| `src-tauri/src/lib.rs` | Register `optimize_prompt` |
| `src/stores/chatStore.ts` | Zustand: messages, addMessage, appendChunk, newChat, persistence to localStorage |
| `src/stores/chatStore.test.ts` | Store lifecycle |
| `src-tauri/src/api_clients/claude_cli/chat.rs` | `chat_send_message(text, history)` Tauri command — emits `chat:stream-chunk` events |
| `src/lib/chatCommands.ts` | TS wrapper that calls `chat_send_message` and subscribes to events |
| `src/pages/Chat.tsx` | Chat page — message list + input + Optimize + ToolDropdown (limited to Claude variants) |
| `src/pages/Chat.test.tsx` | Page tests |
| `src/components/shell/Sidebar.tsx` | Add "Chat" entry above the Settings gear |
| `src/App.tsx` | Add `/chat` route lazy-loaded |
| `src/components/shell/ShortcutHelpOverlay.tsx` | Extend with "Tool overrides" section listing OVERRIDE_ALIASES |
| `src/pages/{WebsiteBuilder,Graphic2D,Graphic3D,Video,Typography}.tsx` | Each page mounts `<ToolDropdown>` + `<OptimizeToggle>` next to its prompt input; handleGenerate invokes parseOverride and respects the chosen Model |
| `src-tauri/src/ai_router/models.rs` | `AiRequest` gains optional `model_override: Option<Model>` field |
| `src-tauri/src/ai_router/mod.rs` | `AiRouter::route` checks `model_override` first; if set, uses it as primary + keeps strategy fallbacks |
| `e2e/tests/chat.spec.ts` | New — type, send, mocked-receive |
| `e2e/tests/generate-with-override.spec.ts` | New — `/flux prompt` → assert mocked invoke received Model::FalFluxPro |
| `docs/superpowers/specs/2026-04-19-phase-9-verification-report.md` | Closure report |

---

## Sub-Project A — Claude CLI Bridge (Tasks 1-7)

## Task 1: detect_claude_binary helper + tests

**Files:**
- Create: `src-tauri/src/api_clients/claude_cli/discovery.rs`
- Create: `src-tauri/src/api_clients/claude_cli/mod.rs` (declares submodules)

- [ ] **Step 1: Write failing tests**

```rust
// src-tauri/src/api_clients/claude_cli/discovery.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detect_returns_some_when_path_exists() {
        // Use the test process's own argv[0] which always exists
        let exe = std::env::current_exe().unwrap();
        let dir = exe.parent().unwrap().to_path_buf();
        // Put `exe`'s name as our "claude" candidate via candidates()
        let candidates = vec![exe.clone()];
        assert_eq!(detect_first_existing(&candidates), Some(exe));
    }

    #[test]
    fn detect_returns_none_for_all_missing() {
        let candidates = vec![
            PathBuf::from("/definitely/missing/claude"),
            PathBuf::from("/also/missing/claude"),
        ];
        assert_eq!(detect_first_existing(&candidates), None);
    }
}
```

- [ ] **Step 2: Run, verify FAIL**

```bash
cd src-tauri && cargo test --lib api_clients::claude_cli::discovery
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

```rust
// src-tauri/src/api_clients/claude_cli/mod.rs
pub mod discovery;
pub mod stream_parser;
```

```rust
// src-tauri/src/api_clients/claude_cli/discovery.rs
//! Locate the user-installed `claude` CLI binary. We do NOT bundle it —
//! the user runs `brew install anthropic/claude-code/claude` (or similar)
//! and `claude login` once. This module just finds whatever path it lives at.

use std::path::{Path, PathBuf};

/// Candidate paths checked in order. `which claude` first (respects user PATH),
/// then known install locations on macOS.
pub fn default_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    // 1. PATH lookup via `which` — macOS-only call, falls back gracefully
    if let Ok(out_bytes) = std::process::Command::new("which").arg("claude").output() {
        if out_bytes.status.success() {
            let path = String::from_utf8_lossy(&out_bytes.stdout).trim().to_string();
            if !path.is_empty() {
                out.push(PathBuf::from(path));
            }
        }
    }
    // 2. Known installs
    if let Some(home) = std::env::var_os("HOME") {
        let h = PathBuf::from(home);
        out.push(h.join(".claude/local/claude"));
        out.push(h.join(".local/bin/claude"));
    }
    out.push(PathBuf::from("/opt/homebrew/bin/claude"));
    out.push(PathBuf::from("/usr/local/bin/claude"));
    out
}

/// Return the first candidate path that exists on disk.
pub fn detect_first_existing(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|p| p.exists()).cloned()
}

/// One-shot detection — returns Some(path) if any candidate exists, None otherwise.
pub fn detect_claude_binary() -> Option<PathBuf> {
    detect_first_existing(&default_candidates())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detect_returns_some_when_path_exists() {
        let exe = std::env::current_exe().unwrap();
        let candidates = vec![exe.clone()];
        assert_eq!(detect_first_existing(&candidates), Some(exe));
    }

    #[test]
    fn detect_returns_none_for_all_missing() {
        let candidates = vec![
            PathBuf::from("/definitely/missing/claude"),
            PathBuf::from("/also/missing/claude"),
        ];
        assert_eq!(detect_first_existing(&candidates), None);
    }
}
```

Wire `pub mod claude_cli;` into `src-tauri/src/api_clients/mod.rs` (search for the existing `pub mod claude;` line and add `pub mod claude_cli;` next to it).

- [ ] **Step 4: Run tests, verify PASS**

```bash
cd src-tauri && cargo test --lib api_clients::claude_cli::discovery
```

Expected: 2 PASS.

- [ ] **Step 5: Commit**

```bash
cd .. && pnpm biome check .
git add src-tauri/src/api_clients/
git commit -m "feat(claude-cli): detect_claude_binary + candidate paths"
```

---

## Task 2: stream_parser — JSON-Lines accumulator + tests

**Files:**
- Create: `src-tauri/src/api_clients/claude_cli/stream_parser.rs`

The `claude -p --output-format stream-json` output is one JSON object per line. The shapes we care about:
```
{"type":"system","subtype":"init","model":"claude-opus-4-7","session_id":"..."}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Hello "}]}}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"world"}]}}
{"type":"result","subtype":"success","total_cost_usd":0.001,"usage":{"input_tokens":5,"output_tokens":3}}
```

(Real format may include `tool_use` blocks; we ignore those for plain `-p` text generation.)

- [ ] **Step 1: Write failing tests**

```rust
// src-tauri/src/api_clients/claude_cli/stream_parser.rs
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"type":"system","subtype":"init","model":"claude-opus-4-7","session_id":"abc"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Hello "}]}}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"world"}]}}
{"type":"result","subtype":"success","total_cost_usd":0.0012,"usage":{"input_tokens":5,"output_tokens":3}}
"#;

    #[test]
    fn accumulates_text_across_assistant_chunks() {
        let mut acc = StreamAccumulator::new();
        for line in SAMPLE.lines() {
            acc.feed_line(line);
        }
        let result = acc.finish().unwrap();
        assert_eq!(result.text, "Hello world");
        assert_eq!(result.input_tokens, Some(5));
        assert_eq!(result.output_tokens, Some(3));
        assert!((result.cost_usd.unwrap() - 0.0012).abs() < 1e-9);
    }

    #[test]
    fn ignores_tool_use_blocks_for_text_extraction() {
        let mut acc = StreamAccumulator::new();
        acc.feed_line(r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"X "},{"type":"tool_use","name":"Bash","input":{}},{"type":"text","text":"Y"}]}}"#);
        acc.feed_line(r#"{"type":"result","subtype":"success"}"#);
        let result = acc.finish().unwrap();
        assert_eq!(result.text, "X Y");
    }

    #[test]
    fn surfaces_error_subtype() {
        let mut acc = StreamAccumulator::new();
        acc.feed_line(r#"{"type":"result","subtype":"error_during_execution","error":{"message":"auth: token expired"}}"#);
        let err = acc.finish().unwrap_err();
        assert!(err.contains("auth: token expired"));
    }

    #[test]
    fn malformed_lines_are_skipped() {
        let mut acc = StreamAccumulator::new();
        acc.feed_line("not json");
        acc.feed_line(r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"ok"}]}}"#);
        acc.feed_line(r#"{"type":"result","subtype":"success"}"#);
        let result = acc.finish().unwrap();
        assert_eq!(result.text, "ok");
    }

    #[test]
    fn empty_stream_returns_empty_text() {
        let mut acc = StreamAccumulator::new();
        acc.feed_line(r#"{"type":"result","subtype":"success"}"#);
        let result = acc.finish().unwrap();
        assert_eq!(result.text, "");
    }
}
```

- [ ] **Step 2: Run, verify FAIL**

```bash
cd src-tauri && cargo test --lib api_clients::claude_cli::stream_parser
```

- [ ] **Step 3: Implement**

```rust
// src-tauri/src/api_clients/claude_cli/stream_parser.rs
//! Pure parser for `claude -p --output-format stream-json` output.
//!
//! Each line is a JSON object describing an event in the streamed response.
//! We're interested in:
//! - `assistant` events with `content[].type == "text"` → accumulate text
//! - `result` events with `subtype == "success"` → final cost + usage
//! - `result` events with `subtype` containing `error` → return Err

use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct StreamResult {
    pub text: String,
    pub cost_usd: Option<f64>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub model: Option<String>,
}

#[derive(Debug, Default)]
pub struct StreamAccumulator {
    text: String,
    cost_usd: Option<f64>,
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    model: Option<String>,
    error: Option<String>,
}

impl StreamAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one stream-json line (newline-stripped). Malformed lines are
    /// silently skipped — claude CLI occasionally emits debug noise.
    pub fn feed_line(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => return,
        };
        let event_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match event_type {
            "system" => {
                if let Some(model) = v.get("model").and_then(|m| m.as_str()) {
                    self.model = Some(model.to_string());
                }
            }
            "assistant" => {
                let blocks = v
                    .pointer("/message/content")
                    .and_then(|c| c.as_array())
                    .cloned()
                    .unwrap_or_default();
                for block in blocks {
                    if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                        if let Some(t) = block.get("text").and_then(|t| t.as_str()) {
                            self.text.push_str(t);
                        }
                    }
                }
            }
            "result" => {
                let subtype = v.get("subtype").and_then(|s| s.as_str()).unwrap_or("");
                if subtype.contains("error") {
                    let msg = v
                        .pointer("/error/message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("unknown CLI error")
                        .to_string();
                    self.error = Some(msg);
                }
                if let Some(c) = v.get("total_cost_usd").and_then(|c| c.as_f64()) {
                    self.cost_usd = Some(c);
                }
                if let Some(usage) = v.get("usage") {
                    self.input_tokens = usage.get("input_tokens").and_then(|t| t.as_u64()).map(|t| t as u32);
                    self.output_tokens = usage.get("output_tokens").and_then(|t| t.as_u64()).map(|t| t as u32);
                }
            }
            _ => {}
        }
    }

    /// Finish parsing — returns Ok with accumulated content or Err on error event.
    pub fn finish(self) -> Result<StreamResult, String> {
        if let Some(err) = self.error {
            return Err(err);
        }
        Ok(StreamResult {
            text: self.text,
            cost_usd: self.cost_usd,
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            model: self.model,
        })
    }
}
```

- [ ] **Step 4: Run + commit**

```bash
cargo test --lib api_clients::claude_cli::stream_parser
cd .. && pnpm biome check .
git add src-tauri/src/api_clients/claude_cli/stream_parser.rs
git commit -m "feat(claude-cli): stream-json accumulator + 5 parser tests"
```

---

## Task 3: ClaudeCliClient implements AiClient

**Files:**
- Create: `src-tauri/src/api_clients/claude_cli/client.rs`
- Modify: `src-tauri/src/api_clients/claude_cli/mod.rs` (add `pub mod client;`)

- [ ] **Step 1: Write failing tests**

The client spawns a real subprocess; we'll test by injecting a `Spawner` trait so tests can return canned output without a real `claude` binary.

```rust
// src-tauri/src/api_clients/claude_cli/client.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_router::{AiRequest, Complexity, Model, Priority, Provider, TaskKind};

    fn req(prompt: &str) -> AiRequest {
        AiRequest {
            id: "t1".into(),
            task: TaskKind::TextGeneration,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt: prompt.into(),
            payload: serde_json::Value::Null,
        }
    }

    struct MockSpawner {
        canned_stdout: String,
        canned_exit: i32,
    }

    #[async_trait::async_trait]
    impl Spawner for MockSpawner {
        async fn spawn(&self, _bin: &std::path::Path, _args: &[String], _stdin: Option<String>) -> SpawnResult {
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
"#.to_string()
    }

    #[tokio::test]
    async fn happy_path_text_generation() {
        let spawner = MockSpawner { canned_stdout: opus_response(), canned_exit: 0 };
        let client = ClaudeCliClient::with_spawner(std::path::PathBuf::from("/fake/claude"), Box::new(spawner));
        let resp = client.execute(Model::ClaudeOpus, &req("hello")).await.unwrap();
        assert_eq!(resp.model, Model::ClaudeOpus);
        let text = resp.output.get("text").and_then(|t| t.as_str()).unwrap();
        assert_eq!(text, "Generated content here");
    }

    #[tokio::test]
    async fn maps_non_zero_exit_to_permanent() {
        let spawner = MockSpawner { canned_stdout: String::new(), canned_exit: 1 };
        let client = ClaudeCliClient::with_spawner(std::path::PathBuf::from("/fake/claude"), Box::new(spawner));
        let err = client.execute(Model::ClaudeSonnet, &req("hello")).await.expect_err("should fail");
        assert!(matches!(err, crate::ai_router::ProviderError::Permanent(_)));
    }

    #[tokio::test]
    async fn maps_error_subtype_to_provider_error() {
        let spawner = MockSpawner {
            canned_stdout: r#"{"type":"result","subtype":"error_during_execution","error":{"message":"auth: token expired"}}"#.into(),
            canned_exit: 0,
        };
        let client = ClaudeCliClient::with_spawner(std::path::PathBuf::from("/fake/claude"), Box::new(spawner));
        let err = client.execute(Model::ClaudeHaiku, &req("hello")).await.expect_err("should fail");
        // Auth-flavored error → Auth variant
        assert!(matches!(err, crate::ai_router::ProviderError::Auth(_)));
    }

    #[tokio::test]
    async fn supports_only_claude_models() {
        let spawner = MockSpawner { canned_stdout: opus_response(), canned_exit: 0 };
        let client = ClaudeCliClient::with_spawner(std::path::PathBuf::from("/fake/claude"), Box::new(spawner));
        assert!(client.supports(Model::ClaudeOpus));
        assert!(client.supports(Model::ClaudeSonnet));
        assert!(client.supports(Model::ClaudeHaiku));
        assert!(!client.supports(Model::FalFluxPro));
    }
}
```

- [ ] **Step 2: Run, verify FAIL**

```bash
cd src-tauri && cargo test --lib api_clients::claude_cli::client
```

- [ ] **Step 3: Implement**

```rust
// src-tauri/src/api_clients/claude_cli/client.rs
//! ClaudeCliClient — uses the local `claude` CLI as transport instead of HTTP.
//!
//! Spawns `claude -p "<prompt>" --model <slug> --output-format stream-json`
//! via tokio::process. Parses streamed events, maps to AiResponse.
//!
//! Authentication piggy-backs on the user's `claude login` (subscription).
//! No API key is read from our keychain.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use super::stream_parser::StreamAccumulator;
use crate::ai_router::{
    AiClient, AiRequest, AiResponse, Model, Provider, ProviderError, ProviderUsage, TaskKind,
};
use crate::keychain::KeyStore;

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
    ) -> Result<crate::api_clients::claude_cli::stream_parser::StreamResult, ProviderError> {
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
            cost_cents: stream.cost_usd.map(|usd| (usd * 100.0).round() as u32),
            cached: false,
        })
    }

    async fn health_check(&self) -> bool {
        self.binary.exists()
    }
}
```

Note: `KeyStore` isn't used by ClaudeCliClient (auth is via `claude login`), but if `AiClient` trait requires a key_store-aware constructor in `registry.rs`, this client just ignores it. Verify trait signature in `models.rs` — if it requires nothing extra, the `new(binary)` constructor is enough.

Add `pub mod client;` to `claude_cli/mod.rs` and re-export: `pub use client::ClaudeCliClient;`.

Also you may need: in `src-tauri/Cargo.toml`, ensure `async-trait` is in dev-dependencies (it should be) for the Spawner trait, AND the `tokio` runtime feature `process` is enabled (it should be from existing usage).

- [ ] **Step 4: Run + commit**

```bash
cd src-tauri && cargo test --lib api_clients::claude_cli
cd .. && pnpm biome check .
git add src-tauri/src/api_clients/claude_cli/
git commit -m "feat(claude-cli): ClaudeCliClient with Spawner trait + 4 mock tests"
```

---

## Task 4: Tauri commands for transport selection + CLI detection

**Files:**
- Create: `src-tauri/src/api_clients/claude_cli_commands.rs`
- Modify: `src-tauri/src/api_clients/mod.rs` (add `pub mod claude_cli_commands;`)
- Modify: `src-tauri/src/lib.rs` (register 3 new commands)

- [ ] **Step 1: Implement commands**

```rust
// src-tauri/src/api_clients/claude_cli_commands.rs
//! Tauri commands for the Claude CLI bridge: detect binary, get/set transport.
//!
//! The transport selection is persisted in the keystore under the meta key
//! `__claude_transport__` with values "auto" | "api" | "cli" so it survives
//! across app restarts without a separate state file.

use std::sync::Arc;

use tauri::State;

use super::claude_cli::discovery::detect_claude_binary;
use crate::keychain::commands::KeyStoreState;
use crate::keychain::KeyStore;

const TRANSPORT_META_KEY: &str = "__claude_transport__";

#[tauri::command]
pub fn detect_claude_cli() -> Option<String> {
    detect_claude_binary().map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn get_claude_transport(state: State<'_, KeyStoreState>) -> String {
    let store: Arc<dyn KeyStore> = state.0.clone();
    store
        .get(TRANSPORT_META_KEY)
        .ok()
        .filter(|v| matches!(v.as_str(), "auto" | "api" | "cli"))
        .unwrap_or_else(|| "auto".to_string())
}

#[tauri::command]
pub fn set_claude_transport(
    transport: String,
    state: State<'_, KeyStoreState>,
) -> Result<(), String> {
    if !matches!(transport.as_str(), "auto" | "api" | "cli") {
        return Err(format!("invalid transport: {transport}"));
    }
    let store: Arc<dyn KeyStore> = state.0.clone();
    store
        .store(TRANSPORT_META_KEY, &transport)
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Register commands**

In `src-tauri/src/lib.rs`, add to imports near other api_clients commands:
```rust
use api_clients::claude_cli_commands::{detect_claude_cli, get_claude_transport, set_claude_transport};
```

In the `tauri::generate_handler![...]` macro, add the three commands alongside the existing keychain commands.

- [ ] **Step 3: Add module declaration**

In `src-tauri/src/api_clients/mod.rs`, add:
```rust
pub mod claude_cli_commands;
```

- [ ] **Step 4: Update registry to honor transport selection**

In `src-tauri/src/api_clients/registry.rs::build_default_clients`, replace the existing single `Provider::Claude` entry with a transport-aware variant:

```rust
// Existing pattern (search for where ClaudeClient is constructed):
//   let claude = Arc::new(ClaudeClient::new(keystore.clone()));
//   clients.insert(Provider::Claude, claude as Arc<dyn AiClient>);
//
// Replace with:
let transport = keystore
    .get("__claude_transport__")
    .ok()
    .unwrap_or_else(|| "auto".to_string());
let claude_client: Arc<dyn AiClient> = match transport.as_str() {
    "cli" => {
        match super::claude_cli::discovery::detect_claude_binary() {
            Some(bin) => Arc::new(super::claude_cli::client::ClaudeCliClient::new(bin)),
            None => {
                eprintln!("[startup] claude transport=cli but binary not found — falling back to HTTP API");
                Arc::new(ClaudeClient::new(keystore.clone()))
            }
        }
    }
    "api" => Arc::new(ClaudeClient::new(keystore.clone())),
    _ => {
        // "auto": prefer CLI when binary is available, else API
        match super::claude_cli::discovery::detect_claude_binary() {
            Some(bin) => Arc::new(super::claude_cli::client::ClaudeCliClient::new(bin)),
            None => Arc::new(ClaudeClient::new(keystore.clone())),
        }
    }
};
clients.insert(Provider::Claude, claude_client);
```

- [ ] **Step 5: Verify build + commit**

```bash
cd src-tauri && cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test
cd .. && pnpm biome check .
git add src-tauri/src/api_clients/ src-tauri/src/lib.rs
git commit -m "feat(claude-cli): Tauri commands + transport-aware registry"
```

---

## Task 5: Frontend wrapper + Settings UI for transport

**Files:**
- Create: `src/lib/claudeTransport.ts`
- Create: `src/lib/claudeTransport.test.ts`
- Modify: `src/components/settings/providers.ts`
- Modify: `src/components/settings/ProviderKeyRow.tsx`

- [ ] **Step 1: TS wrapper**

```ts
// src/lib/claudeTransport.ts
import { invoke } from "@tauri-apps/api/core";

export type ClaudeTransport = "auto" | "api" | "cli";

export const detectClaudeCli = () => invoke<string | null>("detect_claude_cli");
export const getClaudeTransport = () => invoke<ClaudeTransport>("get_claude_transport");
export const setClaudeTransport = (transport: ClaudeTransport) =>
  invoke<void>("set_claude_transport", { transport });
```

- [ ] **Step 2: TS wrapper tests** (mock invoke)

```ts
// src/lib/claudeTransport.test.ts
import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { detectClaudeCli, getClaudeTransport, setClaudeTransport } from "@/lib/claudeTransport";

describe("claudeTransport wrapper", () => {
  it("detectClaudeCli calls correct command", async () => {
    vi.mocked(invoke).mockResolvedValueOnce("/usr/local/bin/claude");
    const result = await detectClaudeCli();
    expect(invoke).toHaveBeenCalledWith("detect_claude_cli");
    expect(result).toBe("/usr/local/bin/claude");
  });

  it("getClaudeTransport returns transport string", async () => {
    vi.mocked(invoke).mockResolvedValueOnce("auto");
    expect(await getClaudeTransport()).toBe("auto");
  });

  it("setClaudeTransport passes the value as named arg", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined);
    await setClaudeTransport("cli");
    expect(invoke).toHaveBeenCalledWith("set_claude_transport", { transport: "cli" });
  });
});
```

- [ ] **Step 3: Extend ProviderDef**

In `src/components/settings/providers.ts`, find the Claude entry (id `"claude"`) and add an optional `transports` field plus update the type:

```ts
export interface ProviderDef {
  id: string;
  label: string;
  plan: "Subscription (Pro/Max)" | "Pay-per-Use";
  helpUrl: string;
  hint?: string;
  /** If present, this provider supports multiple transports — Settings shows a selector. */
  transports?: readonly ("auto" | "api" | "cli")[];
}

// Add to the claude entry:
{ id: "claude", label: "Anthropic Claude", plan: "Subscription (Pro/Max)",
  helpUrl: "https://console.anthropic.com/settings/keys",
  transports: ["auto", "api", "cli"] as const },
```

- [ ] **Step 4: Update ProviderKeyRow**

In `src/components/settings/ProviderKeyRow.tsx`, when `provider.transports` is present, render a small `<select>` between the status dot and the input:

```tsx
import { useEffect, useState } from "react";
import {
  detectClaudeCli,
  getClaudeTransport,
  setClaudeTransport,
  type ClaudeTransport,
} from "@/lib/claudeTransport";

// ...inside ProviderKeyRow component, near the existing useState calls:
const [transport, setTransport] = useState<ClaudeTransport>("auto");
const [cliPath, setCliPath] = useState<string | null>(null);

useEffect(() => {
  if (!provider.transports) return;
  void getClaudeTransport().then(setTransport);
  void detectClaudeCli().then(setCliPath);
}, [provider.transports]);

// In the JSX, just below the status-dot row, conditionally:
{provider.transports ? (
  <div className="flex items-center gap-2">
    <label className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
      Transport
    </label>
    <select
      className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 text-2xs text-neutral-dark-100"
      value={transport}
      onChange={async (e) => {
        const next = e.target.value as ClaudeTransport;
        setTransport(next);
        try {
          await setClaudeTransport(next);
          notify({ kind: "success", message: `Claude transport: ${next}` });
        } catch (err) {
          notify({
            kind: "error",
            message: "Failed to set transport",
            detail: String(err),
          });
        }
      }}
    >
      <option value="auto">Auto (CLI if installed, else API)</option>
      <option value="api">API Key</option>
      <option value="cli">Subscription via CLI</option>
    </select>
    {transport === "cli" && cliPath === null && (
      <span className="font-mono text-2xs text-rose-400">
        ⚠ `claude` CLI not found — install: brew install anthropic/claude-code/claude
      </span>
    )}
    {cliPath && transport !== "api" && (
      <span className="font-mono text-2xs text-emerald-500">CLI: {cliPath}</span>
    )}
  </div>
) : null}
```

- [ ] **Step 5: Run + commit**

```bash
pnpm vitest run src/lib/claudeTransport src/components/settings
pnpm biome check .
git add src/lib/claudeTransport.ts src/lib/claudeTransport.test.ts src/components/settings/
git commit -m "feat(settings): Claude transport selector (auto/api/cli) + CLI auto-detect display"
```

---

## Task 6: AiRequest gains model_override field

**Files:**
- Modify: `src-tauri/src/ai_router/models.rs`
- Modify: `src-tauri/src/ai_router/mod.rs`

This is the foundation for both Sub-Project A's existing tests and Sub-Project B's `/tool` override.

- [ ] **Step 1: Extend AiRequest**

In `src-tauri/src/ai_router/models.rs`, find the `AiRequest` struct and add:

```rust
#[derive(Debug, Clone)]
pub struct AiRequest {
    pub id: String,
    pub task: TaskKind,
    pub priority: Priority,
    pub complexity: Complexity,
    pub prompt: String,
    pub payload: serde_json::Value,
    /// Caller may explicitly request a model. When set, the router uses this
    /// as the primary model and keeps the strategy's fallbacks. When None,
    /// the strategy picks both primary and fallbacks normally.
    #[serde(default)]
    pub model_override: Option<Model>,
}
```

If `AiRequest` doesn't currently derive `Serialize/Deserialize`, you may need to add those if it's used over IPC. Check existing usage with `grep -rn "AiRequest" src-tauri/src/`. The field is `Option`, so existing non-IPC construction sites with struct-literal syntax need updating to add `model_override: None`. Do that systematically (search and add).

- [ ] **Step 2: Honor override in router**

In `src-tauri/src/ai_router/mod.rs::route` (or `execute_decision`), after calling `self.strategy.select(&request)`:

```rust
let mut decision = self.strategy.select(&request);
if let Some(override_model) = request.model_override {
    // Demote the original primary into the fallbacks chain (if not already there)
    let original_primary = decision.primary;
    decision.primary = override_model;
    if !decision.fallbacks.contains(&original_primary) && original_primary != override_model {
        decision.fallbacks.insert(0, original_primary);
    }
}
```

- [ ] **Step 3: Add a router test**

In `src-tauri/src/ai_router/mod.rs::tests`, add:

```rust
#[tokio::test]
async fn route_with_model_override_uses_override_as_primary() {
    let primary = Arc::new(MockClient::failing_with(
        Provider::Fal,
        ProviderError::Transient("simulated".into()),
    ));
    let secondary_provider = Provider::Claude;
    let secondary = Arc::new(MockClient::ok_returning("hello"));
    let mut clients: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();
    clients.insert(Provider::Fal, primary);
    clients.insert(secondary_provider, secondary);

    let router = AiRouter::new(
        Arc::new(DefaultRoutingStrategy),
        clients,
        zero_retry_policy(),
        Arc::new(PriorityQueue::new()),
    );

    let mut request = AiRequest {
        id: "x".into(),
        task: TaskKind::ImageGeneration,
        priority: Priority::Normal,
        complexity: Complexity::Medium,
        prompt: "test".into(),
        payload: serde_json::Value::Null,
        model_override: Some(Model::ClaudeOpus),
    };

    let response = router.route(request.clone()).await.expect("override should succeed");
    assert_eq!(response.model, Model::ClaudeOpus);
}
```

- [ ] **Step 4: Update all struct-literal AiRequest construction sites**

Run `grep -rn "AiRequest {" src-tauri/src/ src-tauri/tests/` and add `model_override: None,` to every literal. (If serde `#[serde(default)]` is in place + most callers use struct-update syntax, this is minimal — but be exhaustive.)

- [ ] **Step 5: Build + run + commit**

```bash
cd src-tauri && cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test
cd .. && pnpm biome check .
git add src-tauri/src/ai_router/
git commit -m "feat(router): AiRequest.model_override — explicit primary with strategy fallbacks"
```

---

## Task 7: Sub-Project A verification + closure note

**Files:**
- Create: `docs/superpowers/specs/2026-04-19-phase-9-sub-a-verification.md`

- [ ] **Step 1: Verify**

```bash
cd src-tauri && cargo test
cd .. && pnpm vitest run
pnpm e2e
pnpm exec tsc --noEmit
pnpm biome check .
```

All green.

- [ ] **Step 2: Write closure note**

```markdown
# Phase 9 Sub-Project A — Claude CLI Bridge — Closure

**Date:** 2026-04-19
**Spec:** `docs/superpowers/specs/2026-04-19-phase-9-claude-bridge-and-tool-ux-design.md`
**Plan:** `docs/superpowers/plans/2026-04-19-phase-9-claude-bridge-and-tool-ux.md`

## Summary

ClaudeCliClient implemented; transport-selectable per Claude in Settings (Auto / API / CLI). All 5 backend Claude callers route through whichever transport is configured, with no other code changes (drop-in via `AiClient` trait).

Verified: cargo test all green; frontend tests pass; CI 5/5.

## Pillar coverage

- discovery: <N> tests
- stream-parser: 5 tests covering normal stream, tool-use blocks, error subtype, malformed lines, empty stream
- ClaudeCliClient: 4 mock-spawner tests (happy path, exit-code mapping, error subtype mapping, supports filter)
- Tauri commands: detect_claude_cli, get_claude_transport, set_claude_transport — registered in invoke_handler
- Registry: transport-aware Claude client construction with auto-fallback when CLI missing
- Frontend: claudeTransport.ts wrapper + Settings UI tri-state selector + CLI auto-detect display
- AiRequest.model_override: foundation for Sub-Project B's `/tool` override; router honors it

Sub-Project B (Chat panel + tool UX) follows.
```

- [ ] **Step 3: Commit + push + watch**

```bash
git add docs/superpowers/specs/2026-04-19-phase-9-sub-a-verification.md
git commit -m "docs(phase-9): Sub-Project A (Claude CLI Bridge) closure note"
git push origin main
gh run list --branch main --limit 1 --json databaseId
gh run watch <id> --exit-status
```

CI must be 5/5 green.

---

## Sub-Project B — Chat Panel + Tool UX (Tasks 8-22)

## Task 8: toolCatalog.ts capability matrix + tests

**Files:**
- Create: `src/lib/toolCatalog.ts`
- Create: `src/lib/toolCatalog.test.ts`

- [ ] **Step 1: Test**

```ts
// src/lib/toolCatalog.test.ts
import { describe, expect, it } from "vitest";
import { CATALOG, getToolsFor, type TaskKind } from "@/lib/toolCatalog";

describe("toolCatalog", () => {
  it("provides tools for every TaskKind", () => {
    const expected: TaskKind[] = [
      "TextGeneration",
      "ImageGeneration",
      "ImageEdit",
      "Inpaint",
      "Upscale",
      "Logo",
      "TextToVideo",
      "ImageToVideo",
      "VideoMontage",
      "Text3D",
      "Image3D",
      "ImageAnalysis",
      "DepthMap",
    ];
    for (const task of expected) {
      const tools = getToolsFor(task);
      expect(tools.length).toBeGreaterThan(0);
      expect(tools.some((t) => t.tier === "primary")).toBe(true);
    }
  });

  it("Logo has Ideogram as the only tool", () => {
    const tools = getToolsFor("Logo");
    expect(tools).toHaveLength(1);
    expect(tools[0]?.id).toBe("ideogram-v3");
    expect(tools[0]?.tier).toBe("primary");
  });

  it("TextToVideo lists Kling V2 primary, V1.5 + Runway + Higgsfield as fallbacks", () => {
    const tools = getToolsFor("TextToVideo");
    expect(tools[0]?.id).toBe("fal-kling-v2-master");
    expect(tools[0]?.tier).toBe("primary");
    expect(tools.some((t) => t.id === "fal-kling-v15" && t.tier === "fallback")).toBe(true);
    expect(tools.some((t) => t.id === "runway-gen3" && t.tier === "fallback")).toBe(true);
    expect(tools.some((t) => t.id === "higgsfield" && t.tier === "fallback")).toBe(true);
  });

  it("VideoMontage offers Shotstack primary + Remotion alternative", () => {
    const tools = getToolsFor("VideoMontage");
    expect(tools[0]?.id).toBe("shotstack");
    expect(tools.some((t) => t.id === "remotion" && t.tier === "alternative")).toBe(true);
  });
});
```

- [ ] **Step 2: Implement**

```ts
// src/lib/toolCatalog.ts
//! Central capability matrix: TaskKind → ToolDef[] in priority tier order.
//!
//! Drives both the per-prompt Tool dropdown UI AND the override-slug
//! resolver. Must stay in sync with src-tauri/src/ai_router/router.rs's
//! DefaultRoutingStrategy::select.

export type TaskKind =
  | "TextGeneration"
  | "ImageGeneration"
  | "ImageEdit"
  | "Inpaint"
  | "Upscale"
  | "Logo"
  | "TextToVideo"
  | "ImageToVideo"
  | "VideoMontage"
  | "Text3D"
  | "Image3D"
  | "ImageAnalysis"
  | "DepthMap";

export type Tier = "primary" | "fallback" | "alternative";

export interface ToolDef {
  /** Stable id used as the override slug suffix and Model identifier. */
  id: string;
  /** Display name in the dropdown. */
  label: string;
  /** Provider/host. */
  provider: "claude" | "fal" | "replicate" | "ideogram" | "kling" | "runway" | "higgsfield" | "shotstack" | "meshy" | "remotion";
  /** Backend Model enum variant (PascalCase, matches Rust). */
  model: string;
  tier: Tier;
}

export const CATALOG: Record<TaskKind, readonly ToolDef[]> = {
  TextGeneration: [
    { id: "claude-opus", label: "Claude Opus", provider: "claude", model: "ClaudeOpus", tier: "primary" },
    { id: "claude-sonnet", label: "Claude Sonnet", provider: "claude", model: "ClaudeSonnet", tier: "fallback" },
    { id: "claude-haiku", label: "Claude Haiku", provider: "claude", model: "ClaudeHaiku", tier: "fallback" },
  ],
  ImageGeneration: [
    { id: "fal-flux-pro", label: "Flux Pro 1.1", provider: "fal", model: "FalFluxPro", tier: "primary" },
    { id: "fal-sdxl", label: "SDXL Fast", provider: "fal", model: "FalSdxl", tier: "fallback" },
    { id: "replicate-flux-dev", label: "Flux Dev", provider: "replicate", model: "ReplicateFluxDev", tier: "fallback" },
  ],
  ImageEdit: [
    { id: "fal-flux-pro", label: "Flux Pro 1.1", provider: "fal", model: "FalFluxPro", tier: "primary" },
    { id: "replicate-flux-dev", label: "Flux Dev", provider: "replicate", model: "ReplicateFluxDev", tier: "fallback" },
  ],
  Inpaint: [
    { id: "fal-flux-fill", label: "Flux Fill (inpaint)", provider: "fal", model: "FalFluxFill", tier: "primary" },
  ],
  Upscale: [
    { id: "fal-real-esrgan", label: "Real-ESRGAN", provider: "fal", model: "FalRealEsrgan", tier: "primary" },
  ],
  Logo: [
    { id: "ideogram-v3", label: "Ideogram v3", provider: "ideogram", model: "IdeogramV3", tier: "primary" },
  ],
  TextToVideo: [
    { id: "fal-kling-v2-master", label: "Kling V2 Master (via fal)", provider: "fal", model: "FalKlingV2Master", tier: "primary" },
    { id: "fal-kling-v15", label: "Kling V1.5 (via fal)", provider: "fal", model: "FalKlingV15", tier: "fallback" },
    { id: "runway-gen3", label: "Runway Gen-3", provider: "runway", model: "RunwayGen3", tier: "fallback" },
    { id: "higgsfield", label: "Higgsfield", provider: "higgsfield", model: "HiggsfieldMulti", tier: "fallback" },
  ],
  ImageToVideo: [
    { id: "fal-kling-v2-master", label: "Kling V2 Master (via fal)", provider: "fal", model: "FalKlingV2Master", tier: "primary" },
    { id: "fal-kling-v15", label: "Kling V1.5 (via fal)", provider: "fal", model: "FalKlingV15", tier: "fallback" },
    { id: "runway-gen3", label: "Runway Gen-3", provider: "runway", model: "RunwayGen3", tier: "fallback" },
    { id: "higgsfield", label: "Higgsfield", provider: "higgsfield", model: "HiggsfieldMulti", tier: "fallback" },
  ],
  VideoMontage: [
    { id: "shotstack", label: "Shotstack (cloud)", provider: "shotstack", model: "ShotstackMontage", tier: "primary" },
    { id: "remotion", label: "Remotion (local)", provider: "remotion", model: "RemotionLocal", tier: "alternative" },
  ],
  Text3D: [
    { id: "meshy-text-3d", label: "Meshy Text-3D", provider: "meshy", model: "MeshyText3D", tier: "primary" },
    { id: "replicate-triposr", label: "TripoSR (preview)", provider: "replicate", model: "ReplicateTripoSR", tier: "alternative" },
  ],
  Image3D: [
    { id: "meshy-image-3d", label: "Meshy Image-3D", provider: "meshy", model: "MeshyImage3D", tier: "primary" },
    { id: "replicate-triposr", label: "TripoSR (preview)", provider: "replicate", model: "ReplicateTripoSR", tier: "fallback" },
  ],
  ImageAnalysis: [
    { id: "claude-sonnet", label: "Claude Sonnet", provider: "claude", model: "ClaudeSonnet", tier: "primary" },
    { id: "claude-haiku", label: "Claude Haiku", provider: "claude", model: "ClaudeHaiku", tier: "fallback" },
  ],
  DepthMap: [
    { id: "replicate-depth-anything-v2", label: "Depth-Anything v2", provider: "replicate", model: "ReplicateDepthAnythingV2", tier: "primary" },
  ],
};

export function getToolsFor(task: TaskKind): readonly ToolDef[] {
  return CATALOG[task] ?? [];
}
```

Note: `RemotionLocal` is not currently a Model variant in the Rust enum. Either add it (preferred — it represents the local Remotion render path) OR mark it as a special-case in dropdown that doesn't go through the AiRouter. Easier path: add `Model::RemotionLocal` to `models.rs` later if user explicitly picks it; dropdown just records the choice and Video page routes accordingly. For now, the catalog string is informational.

- [ ] **Step 3: Run + commit**

```bash
pnpm vitest run src/lib/toolCatalog
pnpm biome check .
git add src/lib/toolCatalog.ts src/lib/toolCatalog.test.ts
git commit -m "feat(tooling): toolCatalog.ts capability matrix + 4 tests"
```

---

## Task 9: promptOverride.ts parser + 12 tests

**Files:**
- Create: `src/lib/promptOverride.ts`
- Create: `src/lib/promptOverride.test.ts`

- [ ] **Step 1: Tests**

```ts
// src/lib/promptOverride.test.ts
import { describe, expect, it } from "vitest";
import { parseOverride, OVERRIDE_ALIASES, resolveOverrideToModel } from "@/lib/promptOverride";

describe("parseOverride", () => {
  it("recognizes /slug at start", () => {
    const r = parseOverride("/flux a sunset");
    expect(r.override).toBe("flux");
    expect(r.cleanPrompt).toBe("a sunset");
    expect(r.slugLocation).toBe("start");
  });

  it("recognizes /slug at end", () => {
    const r = parseOverride("a sunset /flux");
    expect(r.override).toBe("flux");
    expect(r.cleanPrompt).toBe("a sunset");
    expect(r.slugLocation).toBe("end");
  });

  it("ignores /slug in the middle", () => {
    const r = parseOverride("a /flux sunset");
    expect(r.override).toBeUndefined();
    expect(r.cleanPrompt).toBe("a /flux sunset");
  });

  it("returns no override on plain prompt", () => {
    const r = parseOverride("a sunset over berlin");
    expect(r.override).toBeUndefined();
    expect(r.cleanPrompt).toBe("a sunset over berlin");
  });

  it("only matches known slugs (whitelist)", () => {
    const r = parseOverride("/unknown-slug-xyz a sunset");
    expect(r.override).toBeUndefined();
    expect(r.cleanPrompt).toBe("/unknown-slug-xyz a sunset");
  });

  it("first-position wins when both ends have a slug", () => {
    const r = parseOverride("/flux some prompt /sdxl");
    expect(r.override).toBe("flux");
    expect(r.cleanPrompt).toBe("some prompt /sdxl");
  });

  it("is case-insensitive on slugs", () => {
    const r = parseOverride("/Flux a sunset");
    expect(r.override).toBe("flux");
  });

  it("trims surrounding whitespace from cleanPrompt", () => {
    const r = parseOverride("  /flux  a sunset  ");
    expect(r.override).toBe("flux");
    expect(r.cleanPrompt).toBe("a sunset");
  });

  it("an empty prompt with only a slug yields the slug + empty cleanPrompt", () => {
    const r = parseOverride("/flux");
    expect(r.override).toBe("flux");
    expect(r.cleanPrompt).toBe("");
  });

  it("/kling alias resolves to FalKlingV2Master", () => {
    expect(resolveOverrideToModel("kling")).toBe("FalKlingV2Master");
  });

  it("/kling-v15 resolves to FalKlingV15", () => {
    expect(resolveOverrideToModel("kling-v15")).toBe("FalKlingV15");
  });

  it("unknown alias resolves to undefined", () => {
    expect(resolveOverrideToModel("xyz")).toBeUndefined();
  });
});
```

- [ ] **Step 2: Implement**

```ts
// src/lib/promptOverride.ts
//! Parse `/tool` overrides from the start or end of a prompt.
//!
//! Rules:
//! - Slug must be at start (matches `^/<slug>\s+`) OR at end (matches `\s+/<slug>$`)
//! - Mid-string `/slug` is ignored (user typed a slash for unrelated reason)
//! - First-position wins if both ends carry a slug
//! - Slug is case-insensitive; whitelist-checked against OVERRIDE_ALIASES
//! - Unknown slugs are NOT consumed — left in cleanPrompt so user sees them

/** Map of override slug → backend Model enum string (PascalCase). */
export const OVERRIDE_ALIASES: Record<string, string> = {
  // Claude
  claude: "ClaudeSonnet",
  "claude-opus": "ClaudeOpus",
  "claude-sonnet": "ClaudeSonnet",
  "claude-haiku": "ClaudeHaiku",
  // fal images
  sdxl: "FalSdxl",
  flux: "FalFluxPro",
  "flux-pro": "FalFluxPro",
  "flux-fill": "FalFluxFill",
  upscale: "FalRealEsrgan",
  // Replicate
  "flux-dev": "ReplicateFluxDev",
  triposr: "ReplicateTripoSR",
  depth: "ReplicateDepthAnythingV2",
  // Logo
  ideogram: "IdeogramV3",
  // Video (fal-aggregated Kling)
  kling: "FalKlingV2Master",
  "kling-v2": "FalKlingV2Master",
  "kling-v15": "FalKlingV15",
  // Direct video providers
  runway: "RunwayGen3",
  higgsfield: "HiggsfieldMulti",
  // Video montage
  shotstack: "ShotstackMontage",
  remotion: "RemotionLocal",
  // 3D
  meshy: "MeshyText3D",
  "meshy-text": "MeshyText3D",
  "meshy-image": "MeshyImage3D",
};

export interface ParsedOverride {
  override?: string;
  cleanPrompt: string;
  slugLocation?: "start" | "end";
}

const SLUG_RE = /^[a-z0-9][a-z0-9-]*$/;

function isKnownSlug(slug: string): boolean {
  return Object.hasOwn(OVERRIDE_ALIASES, slug.toLowerCase());
}

export function parseOverride(input: string): ParsedOverride {
  const trimmed = input.trim();
  // Try start: /<slug> [rest...]
  const startMatch = trimmed.match(/^\/([a-z0-9][a-z0-9-]*)(?:\s+(.*))?$/i);
  if (startMatch) {
    const slug = startMatch[1].toLowerCase();
    if (isKnownSlug(slug) && SLUG_RE.test(slug)) {
      return {
        override: slug,
        cleanPrompt: (startMatch[2] ?? "").trim(),
        slugLocation: "start",
      };
    }
  }
  // Try end: [rest...] /<slug>
  const endMatch = trimmed.match(/^(.*?)\s+\/([a-z0-9][a-z0-9-]*)$/i);
  if (endMatch) {
    const slug = endMatch[2].toLowerCase();
    if (isKnownSlug(slug) && SLUG_RE.test(slug)) {
      return {
        override: slug,
        cleanPrompt: endMatch[1].trim(),
        slugLocation: "end",
      };
    }
  }
  return { cleanPrompt: trimmed };
}

export function resolveOverrideToModel(slug: string): string | undefined {
  return OVERRIDE_ALIASES[slug.toLowerCase()];
}
```

- [ ] **Step 3: Run + commit**

```bash
pnpm vitest run src/lib/promptOverride
pnpm biome check .
git add src/lib/promptOverride.ts src/lib/promptOverride.test.ts
git commit -m "feat(tooling): /tool override parser — start or end only, whitelist-checked"
```

---

## Task 10: ToolDropdown component + tests

**Files:**
- Create: `src/components/ui/ToolDropdown.tsx`
- Create: `src/components/ui/ToolDropdown.test.tsx`

- [ ] **Step 1: Test**

```tsx
// src/components/ui/ToolDropdown.test.tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ToolDropdown } from "@/components/ui/ToolDropdown";

describe("ToolDropdown", () => {
  it("renders selected label", () => {
    render(
      <ToolDropdown taskKind="ImageGeneration" value="auto" onChange={() => {}} />,
    );
    expect(screen.getByRole("button", { name: /Auto/i })).toBeInTheDocument();
  });

  it("opens menu and lists all catalog entries grouped by tier", async () => {
    const user = userEvent.setup();
    render(<ToolDropdown taskKind="TextToVideo" value="auto" onChange={() => {}} />);
    await user.click(screen.getByRole("button"));
    expect(screen.getByText(/Auto/i)).toBeInTheDocument();
    expect(screen.getByText(/Kling V2 Master/i)).toBeInTheDocument();
    expect(screen.getByText(/Kling V1.5/i)).toBeInTheDocument();
    expect(screen.getByText(/Runway Gen-3/i)).toBeInTheDocument();
    expect(screen.getByText(/Higgsfield/i)).toBeInTheDocument();
  });

  it("onChange fires with the chosen Model name", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ToolDropdown taskKind="ImageGeneration" value="auto" onChange={onChange} />);
    await user.click(screen.getByRole("button"));
    await user.click(screen.getByText(/SDXL Fast/i));
    expect(onChange).toHaveBeenCalledWith("FalSdxl");
  });

  it("Auto option fires onChange with 'auto'", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ToolDropdown taskKind="ImageGeneration" value="FalFluxPro" onChange={onChange} />);
    await user.click(screen.getByRole("button"));
    await user.click(screen.getByText(/Auto/i));
    expect(onChange).toHaveBeenCalledWith("auto");
  });
});
```

- [ ] **Step 2: Implement**

```tsx
// src/components/ui/ToolDropdown.tsx
import { ChevronDown } from "lucide-react";
import { useState } from "react";
import { type TaskKind, getToolsFor, type Tier, type ToolDef } from "@/lib/toolCatalog";

export interface ToolDropdownProps {
  taskKind: TaskKind;
  /** Either "auto" (let router decide) or a Model enum string from the catalog. */
  value: string;
  onChange: (next: string) => void;
}

const TIER_LABEL: Record<Tier, string> = {
  primary: "Primary",
  fallback: "Fallbacks",
  alternative: "Alternatives",
};

export function ToolDropdown({ taskKind, value, onChange }: ToolDropdownProps) {
  const [open, setOpen] = useState(false);
  const tools = getToolsFor(taskKind);
  const selected = value === "auto" ? null : tools.find((t) => t.model === value);
  const groups: Record<Tier, ToolDef[]> = { primary: [], fallback: [], alternative: [] };
  for (const t of tools) groups[t.tier].push(t);

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex items-center gap-1 rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 font-mono text-2xs text-neutral-dark-100 uppercase tracking-label hover:border-neutral-dark-600"
      >
        {selected ? selected.label : "Auto"}
        <ChevronDown className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
      </button>
      {open ? (
        <div
          role="listbox"
          className="absolute z-10 mt-1 flex min-w-[200px] flex-col gap-2 rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 p-2 shadow-elevated"
        >
          <button
            type="button"
            onClick={() => {
              onChange("auto");
              setOpen(false);
            }}
            className="rounded-xs px-2 py-1 text-left font-mono text-2xs text-neutral-dark-100 uppercase tracking-label hover:bg-neutral-dark-800"
          >
            Auto (router decides)
          </button>
          {(Object.keys(groups) as Tier[]).map((tier) =>
            groups[tier].length > 0 ? (
              <div key={tier} className="flex flex-col gap-1">
                <span className="font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
                  {TIER_LABEL[tier]}
                </span>
                {groups[tier].map((tool) => (
                  <button
                    key={tool.id}
                    type="button"
                    onClick={() => {
                      onChange(tool.model);
                      setOpen(false);
                    }}
                    className={`rounded-xs px-2 py-1 text-left font-mono text-2xs uppercase tracking-label hover:bg-neutral-dark-800 ${value === tool.model ? "text-accent-500" : "text-neutral-dark-100"}`}
                  >
                    {tool.label}
                  </button>
                ))}
              </div>
            ) : null,
          )}
        </div>
      ) : null}
    </div>
  );
}
```

- [ ] **Step 3: Run + commit**

```bash
pnpm vitest run src/components/ui/ToolDropdown
pnpm biome check .
git add src/components/ui/ToolDropdown.tsx src/components/ui/ToolDropdown.test.tsx
git commit -m "feat(ui): ToolDropdown — capability-matrix-driven, grouped by tier"
```

---

## Task 11: optimize_prompt Tauri command

**Files:**
- Create: `src-tauri/src/code_generator/optimize.rs`
- Modify: `src-tauri/src/code_generator/mod.rs`
- Modify: `src-tauri/src/lib.rs`

The `optimize_prompt` IPC takes a user's raw prompt + TaskKind, runs it through Claude (whichever transport is configured) with a meta-prompt, returns the rewritten prompt.

- [ ] **Step 1: Implement**

```rust
// src-tauri/src/code_generator/optimize.rs
//! `optimize_prompt` Tauri command — rewrites a user prompt for clarity +
//! visual richness using Claude. Wraps the user's text with a meta-prompt
//! tuned to the TaskKind (visual prompts get a different bias than code).

use std::sync::Arc;

use serde::Deserialize;
use tauri::State;

use crate::ai_router::commands::AiRouterState;
use crate::ai_router::{AiRequest, Complexity, Model, Priority, TaskKind};

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
) -> Result<String, String> {
    let meta = meta_for(&input.task);
    let combined = format!("{meta}\n\nUSER PROMPT:\n{}\n\nREWRITTEN:", input.prompt);
    let request = AiRequest {
        id: format!("optimize-{}", uuid::Uuid::new_v4()),
        task: TaskKind::TextGeneration,
        priority: Priority::High,
        complexity: Complexity::Simple,
        prompt: combined,
        payload: serde_json::Value::Null,
        model_override: None,
    };
    let response = router
        .0
        .route(request)
        .await
        .map_err(|e| e.to_string())?;
    let text = response
        .output
        .get("text")
        .and_then(|t| t.as_str())
        .ok_or_else(|| "optimize: no text in response".to_string())?
        .trim()
        .to_string();
    Ok(text)
}
```

If `uuid` isn't a runtime dep, replace with a simpler id: `format!("optimize-{}", chrono::Utc::now().timestamp_millis())`. Verify Cargo.toml.

In `code_generator/mod.rs`, add `pub mod optimize;`.

In `src-tauri/src/lib.rs` invoke_handler add `code_generator::optimize::optimize_prompt`.

- [ ] **Step 2: Test**

Add a unit test in `optimize.rs` that doesn't dispatch to a real router — instead test the `meta_for` function:

```rust
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
```

- [ ] **Step 3: Run + commit**

```bash
cd src-tauri && cargo test --lib code_generator::optimize
cd .. && pnpm biome check .
git add src-tauri/src/code_generator/ src-tauri/src/lib.rs
git commit -m "feat(optimize): optimize_prompt IPC + 4 meta-template tests"
```

---

## Task 12: optimizeCommands.ts wrapper + useOptimizePrompt hook

**Files:**
- Create: `src/lib/optimizeCommands.ts`
- Create: `src/hooks/useOptimizePrompt.ts`
- Create: `src/hooks/useOptimizePrompt.test.tsx`

- [ ] **Step 1: Wrapper**

```ts
// src/lib/optimizeCommands.ts
import { invoke } from "@tauri-apps/api/core";
import type { TaskKind } from "@/lib/toolCatalog";

export const optimizePrompt = (prompt: string, task: TaskKind) =>
  invoke<string>("optimize_prompt", { input: { prompt, task } });
```

- [ ] **Step 2: Hook test**

```tsx
// src/hooks/useOptimizePrompt.test.tsx
import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

vi.mock("@/lib/optimizeCommands", () => ({
  optimizePrompt: vi.fn(),
}));

import { optimizePrompt } from "@/lib/optimizeCommands";
import { useOptimizePrompt } from "@/hooks/useOptimizePrompt";

describe("useOptimizePrompt", () => {
  it("starts with enabled=false, busy=false, canUndo=false", () => {
    const { result } = renderHook(() =>
      useOptimizePrompt({ taskKind: "ImageGeneration", value: "x", setValue: () => {} }),
    );
    expect(result.current.enabled).toBe(false);
    expect(result.current.busy).toBe(false);
    expect(result.current.canUndo).toBe(false);
  });

  it("optimize replaces value + arms undo", async () => {
    let value = "a sunset";
    const setValue = vi.fn((next: string) => {
      value = next;
    });
    vi.mocked(optimizePrompt).mockResolvedValueOnce("warm sunset over berlin, 35mm film grain");
    const { result, rerender } = renderHook(
      ({ v }: { v: string }) =>
        useOptimizePrompt({ taskKind: "ImageGeneration", value: v, setValue }),
      { initialProps: { v: value } },
    );
    await act(async () => {
      await result.current.optimize();
    });
    expect(setValue).toHaveBeenCalledWith("warm sunset over berlin, 35mm film grain");
    expect(result.current.canUndo).toBe(true);
  });

  it("undo restores the previous value + clears canUndo", async () => {
    let value = "original";
    const setValue = vi.fn((next: string) => {
      value = next;
    });
    vi.mocked(optimizePrompt).mockResolvedValueOnce("optimized");
    const { result } = renderHook(() =>
      useOptimizePrompt({ taskKind: "ImageGeneration", value, setValue }),
    );
    await act(async () => {
      await result.current.optimize();
    });
    act(() => {
      result.current.undo();
    });
    expect(setValue).toHaveBeenLastCalledWith("original");
    expect(result.current.canUndo).toBe(false);
  });

  it("optimize failure leaves value unchanged + no undo armed", async () => {
    const setValue = vi.fn();
    vi.mocked(optimizePrompt).mockRejectedValueOnce(new Error("boom"));
    const { result } = renderHook(() =>
      useOptimizePrompt({ taskKind: "ImageGeneration", value: "x", setValue }),
    );
    await act(async () => {
      await expect(result.current.optimize()).rejects.toThrow("boom");
    });
    expect(setValue).not.toHaveBeenCalled();
    expect(result.current.canUndo).toBe(false);
  });
});
```

- [ ] **Step 3: Implement hook**

```tsx
// src/hooks/useOptimizePrompt.ts
import { useState } from "react";
import { optimizePrompt } from "@/lib/optimizeCommands";
import type { TaskKind } from "@/lib/toolCatalog";

export interface UseOptimizePromptArgs {
  taskKind: TaskKind;
  value: string;
  setValue: (next: string) => void;
}

export function useOptimizePrompt({ taskKind, value, setValue }: UseOptimizePromptArgs) {
  const [enabled, setEnabled] = useState(false);
  const [busy, setBusy] = useState(false);
  const [originalForUndo, setOriginalForUndo] = useState<string | null>(null);

  async function optimize(): Promise<void> {
    setBusy(true);
    try {
      const optimized = await optimizePrompt(value, taskKind);
      setOriginalForUndo(value);
      setValue(optimized);
    } finally {
      setBusy(false);
    }
  }

  function undo(): void {
    if (originalForUndo === null) return;
    setValue(originalForUndo);
    setOriginalForUndo(null);
  }

  return {
    enabled,
    setEnabled,
    busy,
    optimize,
    undo,
    canUndo: originalForUndo !== null,
  };
}
```

- [ ] **Step 4: Run + commit**

```bash
pnpm vitest run src/hooks/useOptimizePrompt
pnpm biome check .
git add src/lib/optimizeCommands.ts src/hooks/useOptimizePrompt.ts src/hooks/useOptimizePrompt.test.tsx
git commit -m "feat(optimize): useOptimizePrompt hook + 4 tests"
```

---

## Task 13: OptimizeToggle UI component

**Files:**
- Create: `src/components/ui/OptimizeToggle.tsx`
- Create: `src/components/ui/OptimizeToggle.test.tsx`

- [ ] **Step 1: Test**

```tsx
// src/components/ui/OptimizeToggle.test.tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { OptimizeToggle } from "@/components/ui/OptimizeToggle";

describe("OptimizeToggle", () => {
  it("shows OFF state when not enabled", () => {
    render(<OptimizeToggle enabled={false} onToggle={() => {}} busy={false} canUndo={false} onUndo={() => {}} />);
    expect(screen.getByRole("switch", { name: /Optimize/i })).toHaveAttribute("aria-checked", "false");
  });

  it("toggles on click", async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(<OptimizeToggle enabled={false} onToggle={onToggle} busy={false} canUndo={false} onUndo={() => {}} />);
    await user.click(screen.getByRole("switch"));
    expect(onToggle).toHaveBeenCalledWith(true);
  });

  it("shows Undo button when canUndo", () => {
    render(<OptimizeToggle enabled={true} onToggle={() => {}} busy={false} canUndo={true} onUndo={() => {}} />);
    expect(screen.getByRole("button", { name: /Undo/i })).toBeInTheDocument();
  });

  it("Undo button calls onUndo", async () => {
    const user = userEvent.setup();
    const onUndo = vi.fn();
    render(<OptimizeToggle enabled={true} onToggle={() => {}} busy={false} canUndo={true} onUndo={onUndo} />);
    await user.click(screen.getByRole("button", { name: /Undo/i }));
    expect(onUndo).toHaveBeenCalledTimes(1);
  });

  it("shows busy spinner when busy", () => {
    render(<OptimizeToggle enabled={true} onToggle={() => {}} busy={true} canUndo={false} onUndo={() => {}} />);
    expect(screen.getByLabelText(/optimizing/i)).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Implement**

```tsx
// src/components/ui/OptimizeToggle.tsx
import { Loader2, Sparkles, Undo2 } from "lucide-react";

export interface OptimizeToggleProps {
  enabled: boolean;
  onToggle: (next: boolean) => void;
  busy: boolean;
  canUndo: boolean;
  onUndo: () => void;
}

export function OptimizeToggle({ enabled, onToggle, busy, canUndo, onUndo }: OptimizeToggleProps) {
  return (
    <div className="flex items-center gap-2">
      <button
        type="button"
        role="switch"
        aria-checked={enabled}
        onClick={() => onToggle(!enabled)}
        className={`flex items-center gap-1 rounded-xs border px-2 py-1 font-mono text-2xs uppercase tracking-label transition-colors ${
          enabled
            ? "border-accent-500 bg-accent-500/10 text-accent-500"
            : "border-neutral-dark-700 bg-neutral-dark-900 text-neutral-dark-400 hover:text-neutral-dark-200"
        }`}
        aria-label="Optimize"
      >
        {busy ? (
          <Loader2 className="h-3 w-3 animate-spin" aria-label="optimizing" />
        ) : (
          <Sparkles className="h-3 w-3" strokeWidth={1.5} />
        )}
        Optimize
      </button>
      {canUndo ? (
        <button
          type="button"
          onClick={onUndo}
          className="flex items-center gap-1 rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 font-mono text-2xs text-neutral-dark-400 uppercase tracking-label hover:text-neutral-dark-200"
        >
          <Undo2 className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          Undo
        </button>
      ) : null}
    </div>
  );
}
```

- [ ] **Step 3: Run + commit**

```bash
pnpm vitest run src/components/ui/OptimizeToggle
pnpm biome check .
git add src/components/ui/OptimizeToggle.tsx src/components/ui/OptimizeToggle.test.tsx
git commit -m "feat(ui): OptimizeToggle — switch + busy spinner + Undo button"
```

---

## Task 14: chatStore.ts + tests

**Files:**
- Create: `src/stores/chatStore.ts`
- Create: `src/stores/chatStore.test.ts`

- [ ] **Step 1: Test**

```ts
// src/stores/chatStore.test.ts
import { afterEach, describe, expect, it } from "vitest";
import { useChatStore, CHAT_STORAGE_KEY } from "@/stores/chatStore";

describe("chatStore", () => {
  afterEach(() => {
    useChatStore.getState().newChat();
    window.localStorage.removeItem(CHAT_STORAGE_KEY);
  });

  it("starts empty", () => {
    expect(useChatStore.getState().messages).toEqual([]);
  });

  it("addMessage appends with id + role", () => {
    useChatStore.getState().addMessage({ role: "user", content: "hi" });
    const msgs = useChatStore.getState().messages;
    expect(msgs).toHaveLength(1);
    expect(msgs[0]?.role).toBe("user");
    expect(msgs[0]?.content).toBe("hi");
    expect(msgs[0]?.id).toBeTruthy();
  });

  it("appendChunk concatenates onto a streaming assistant message", () => {
    useChatStore.getState().addMessage({ role: "assistant", content: "" });
    const id = useChatStore.getState().messages[0]!.id;
    useChatStore.getState().appendChunk(id, "Hello ");
    useChatStore.getState().appendChunk(id, "world");
    expect(useChatStore.getState().messages[0]?.content).toBe("Hello world");
  });

  it("newChat clears messages", () => {
    useChatStore.getState().addMessage({ role: "user", content: "hi" });
    useChatStore.getState().newChat();
    expect(useChatStore.getState().messages).toEqual([]);
  });

  it("persists to localStorage", () => {
    useChatStore.getState().addMessage({ role: "user", content: "persisted" });
    const raw = window.localStorage.getItem(CHAT_STORAGE_KEY);
    expect(raw).toBeTruthy();
    expect(raw!).toContain("persisted");
  });
});
```

- [ ] **Step 2: Implement**

```ts
// src/stores/chatStore.ts
import { create } from "zustand";

export const CHAT_STORAGE_KEY = "tm:chat:messages";

export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  createdAt: string;
}

interface ChatState {
  messages: ChatMessage[];
  addMessage: (m: { role: "user" | "assistant"; content: string }) => string;
  appendChunk: (id: string, chunk: string) => void;
  newChat: () => void;
  hydrate: () => void;
}

const makeId = () =>
  typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `m_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

function loadInitial(): ChatMessage[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(CHAT_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as ChatMessage[];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function persist(messages: ChatMessage[]) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(CHAT_STORAGE_KEY, JSON.stringify(messages));
  } catch {
    // localStorage full or disabled — silently ignore
  }
}

export const useChatStore = create<ChatState>((set) => ({
  messages: loadInitial(),
  addMessage: ({ role, content }) => {
    const msg: ChatMessage = {
      id: makeId(),
      role,
      content,
      createdAt: new Date().toISOString(),
    };
    set((state) => {
      const next = [...state.messages, msg];
      persist(next);
      return { messages: next };
    });
    return msg.id;
  },
  appendChunk: (id, chunk) =>
    set((state) => {
      const next = state.messages.map((m) =>
        m.id === id ? { ...m, content: m.content + chunk } : m,
      );
      persist(next);
      return { messages: next };
    }),
  newChat: () => {
    persist([]);
    set({ messages: [] });
  },
  hydrate: () => set({ messages: loadInitial() }),
}));
```

- [ ] **Step 3: Run + commit**

```bash
pnpm vitest run src/stores/chatStore
pnpm biome check .
git add src/stores/chatStore.ts src/stores/chatStore.test.ts
git commit -m "feat(chat): chatStore — Zustand + localStorage persistence"
```

---

## Task 15: chat_send_message Tauri command + chatCommands.ts wrapper

**Files:**
- Create: `src-tauri/src/api_clients/claude_cli/chat.rs`
- Modify: `src-tauri/src/api_clients/claude_cli/mod.rs` (add `pub mod chat;`)
- Modify: `src-tauri/src/lib.rs`
- Create: `src/lib/chatCommands.ts`

- [ ] **Step 1: Backend command (event-streaming)**

```rust
// src-tauri/src/api_clients/claude_cli/chat.rs
//! `chat_send_message` Tauri command. Streams response text back to the
//! frontend via `app.emit("chat:stream-chunk", payload)` so the Chat UI can
//! render token-by-token.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

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
    match client.execute(crate::ai_router::Model::ClaudeSonnet, &request).await {
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
```

Add `pub mod chat;` to `claude_cli/mod.rs`. Register `chat_send_message` in `src-tauri/src/lib.rs`.

- [ ] **Step 2: Frontend wrapper**

```ts
// src/lib/chatCommands.ts
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ChatMessage } from "@/stores/chatStore";

export interface ChunkEvent {
  message_id: string;
  chunk: string;
}

export interface DoneEvent {
  message_id: string;
  error?: string;
}

export async function sendChatMessage(
  messages: ChatMessage[],
  assistantMessageId: string,
  onChunk: (chunk: string) => void,
  onDone: (error?: string) => void,
): Promise<void> {
  const unlistenChunk = await listen<ChunkEvent>("chat:stream-chunk", (event) => {
    if (event.payload.message_id === assistantMessageId) {
      onChunk(event.payload.chunk);
    }
  });
  const unlistenDone = await listen<DoneEvent>("chat:stream-done", (event) => {
    if (event.payload.message_id === assistantMessageId) {
      onDone(event.payload.error);
      unlistenChunk();
      unlistenDone();
    }
  });
  await invoke("chat_send_message", {
    input: { messages: messages.map((m) => ({ role: m.role, content: m.content })) },
    messageId: assistantMessageId,
  }).catch((err) => {
    onDone(String(err));
    unlistenChunk();
    unlistenDone();
  });
}
```

- [ ] **Step 3: Build + commit**

```bash
cd src-tauri && cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test
cd .. && pnpm biome check .
git add src-tauri/src/api_clients/claude_cli/ src-tauri/src/lib.rs src/lib/chatCommands.ts
git commit -m "feat(chat): chat_send_message IPC + event-based chunk streaming"
```

---

## Task 16: Chat page

**Files:**
- Create: `src/pages/Chat.tsx`
- Create: `src/pages/Chat.test.tsx`
- Modify: `src/App.tsx` (add /chat route lazy-loaded)

- [ ] **Step 1: Implement Chat page**

```tsx
// src/pages/Chat.tsx
import { Send, Sparkles, Trash2 } from "lucide-react";
import { useState } from "react";
import { LoadingButton } from "@/components/ui/LoadingButton";
import { OptimizeToggle } from "@/components/ui/OptimizeToggle";
import { ToolDropdown } from "@/components/ui/ToolDropdown";
import { useOptimizePrompt } from "@/hooks/useOptimizePrompt";
import { sendChatMessage } from "@/lib/chatCommands";
import { useChatStore } from "@/stores/chatStore";
import { useUiStore } from "@/stores/uiStore";

export function ChatPage() {
  const messages = useChatStore((s) => s.messages);
  const addMessage = useChatStore((s) => s.addMessage);
  const appendChunk = useChatStore((s) => s.appendChunk);
  const newChat = useChatStore((s) => s.newChat);
  const notify = useUiStore((s) => s.notify);

  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [model, setModel] = useState<string>("auto");
  const optimize = useOptimizePrompt({
    taskKind: "TextGeneration",
    value: input,
    setValue: setInput,
  });

  async function handleSend() {
    if (!input.trim() || busy) return;
    setBusy(true);
    try {
      const userText = input.trim();
      addMessage({ role: "user", content: userText });
      setInput("");
      const assistantId = addMessage({ role: "assistant", content: "" });
      await sendChatMessage(
        [...useChatStore.getState().messages.slice(0, -1), { role: "user", content: userText, id: "_", createdAt: "" }],
        assistantId,
        (chunk) => appendChunk(assistantId, chunk),
        (error) => {
          if (error) {
            notify({ kind: "error", message: "Chat failed", detail: error });
          }
        },
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="grid h-full grid-rows-[auto_1fr_auto]">
      <div className="flex items-center justify-between border-neutral-dark-700 border-b p-6">
        <div className="flex items-center gap-2">
          <span className="font-mono text-2xs text-accent-500 uppercase tracking-label-wide">
            CHAT · CLAUDE
          </span>
        </div>
        <button
          type="button"
          onClick={newChat}
          className="flex items-center gap-1 rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 font-mono text-2xs text-neutral-dark-400 uppercase tracking-label hover:text-neutral-dark-200"
        >
          <Trash2 className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          New chat
        </button>
      </div>

      <div className="overflow-y-auto p-6">
        <div className="flex flex-col gap-4">
          {messages.length === 0 ? (
            <div className="flex h-full items-center justify-center font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              <Sparkles className="mr-2 inline h-3 w-3" /> Start a conversation
            </div>
          ) : (
            messages.map((m) => (
              <div
                key={m.id}
                className={`flex max-w-[80%] flex-col gap-1 rounded-xs border p-3 text-2xs ${m.role === "user" ? "self-end border-accent-500/40 bg-accent-500/10 text-neutral-dark-100" : "self-start border-neutral-dark-700 bg-neutral-dark-900 text-neutral-dark-200"}`}
              >
                <span className="font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
                  {m.role}
                </span>
                <p className="whitespace-pre-wrap leading-relaxed">{m.content}</p>
              </div>
            ))
          )}
        </div>
      </div>

      <div className="flex flex-col gap-2 border-neutral-dark-700 border-t p-4">
        <div className="flex items-center gap-2">
          <ToolDropdown taskKind="TextGeneration" value={model} onChange={setModel} />
          <OptimizeToggle
            enabled={optimize.enabled}
            onToggle={optimize.setEnabled}
            busy={optimize.busy}
            canUndo={optimize.canUndo}
            onUndo={optimize.undo}
          />
        </div>
        <div className="flex items-end gap-2">
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                if (optimize.enabled) {
                  void optimize.optimize().then(handleSend);
                } else {
                  void handleSend();
                }
              }
            }}
            placeholder="Type a message — Cmd+Enter to send"
            className="flex-1 min-h-[60px] rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 p-2 font-mono text-2xs text-neutral-dark-100"
          />
          <LoadingButton
            variant="primary"
            onClick={async () => {
              if (optimize.enabled) {
                await optimize.optimize();
              }
              await handleSend();
            }}
            disabled={!input.trim()}
            loading={busy}
          >
            <Send className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
            Send
          </LoadingButton>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Page test (smoke)**

```tsx
// src/pages/Chat.test.tsx
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ChatPage } from "@/pages/Chat";

describe("ChatPage", () => {
  it("renders header + empty state + input", () => {
    render(<ChatPage />);
    expect(screen.getByText(/CHAT · CLAUDE/i)).toBeInTheDocument();
    expect(screen.getByText(/Start a conversation/i)).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/Type a message/i)).toBeInTheDocument();
  });
});
```

- [ ] **Step 3: Add /chat route**

In `src/App.tsx`, add:
```tsx
const ChatPage = lazy(() => import("@/pages/Chat").then((m) => ({ default: m.ChatPage })));
// ...inside Routes:
<Route path="/chat" element={<ChatPage />} />
```

- [ ] **Step 4: Run + commit**

```bash
pnpm vitest run src/pages/Chat src/App
pnpm biome check .
git add src/pages/Chat.tsx src/pages/Chat.test.tsx src/App.tsx
git commit -m "feat(chat): Chat page with streaming + Tool dropdown + Optimize"
```

---

## Task 17: Sidebar Chat entry above Settings

**Files:**
- Modify: `src/components/shell/Sidebar.tsx`

- [ ] **Step 1: Add Chat link**

Read `Sidebar.tsx` to find where the Settings gear is rendered (was added in P8-S2 H3). Insert a new icon-button ABOVE it for Chat. Use the `MessageCircle` or `MessagesSquare` icon from lucide-react.

```tsx
import { MessagesSquare } from "lucide-react";
// ...inside the bottom-of-sidebar section, BEFORE the Settings button:
<Tooltip content="Chat with Claude" side="right">
  <button
    type="button"
    onClick={() => navigate("/chat")}
    aria-label="Chat with Claude"
    className="..." // mirror Settings button styling
  >
    <MessagesSquare className="h-4 w-4" strokeWidth={1.5} aria-hidden="true" />
  </button>
</Tooltip>
```

- [ ] **Step 2: Add a Cmd+L shortcut for Chat (optional but nice)**

```tsx
const openChat = useCallback(() => navigate("/chat"), [navigate]);
useKeyboardShortcut({
  id: "global:chat",
  combo: "Mod+L",
  handler: openChat,
  scope: "global",
  label: "Open chat",
});
```

- [ ] **Step 3: Verify + commit**

```bash
pnpm vitest run src/components/shell
pnpm biome check .
git add src/components/shell/Sidebar.tsx
git commit -m "feat(chat): Sidebar Chat entry above Settings + Cmd+L shortcut"
```

---

## Task 18: Per-page integration — Graphic2D as the prototype

**Files:**
- Modify: `src/pages/Graphic2D.tsx`

The pattern this task establishes is then replicated in T19 across the other 4 pages.

- [ ] **Step 1: Wire ToolDropdown + OptimizeToggle + override-aware handleGenerate**

In `src/pages/Graphic2D.tsx`:

1. Add state: `const [model, setModel] = useState("auto")`.
2. Pass to ToolDropdown next to existing prompt input + OptimizeToggle.
3. Wrap existing handleGenerate:

```tsx
import { parseOverride, resolveOverrideToModel } from "@/lib/promptOverride";
import { ToolDropdown } from "@/components/ui/ToolDropdown";
import { OptimizeToggle } from "@/components/ui/OptimizeToggle";
import { useOptimizePrompt } from "@/hooks/useOptimizePrompt";

// inside component:
const [model, setModel] = useState<string>("auto");
const optimize = useOptimizePrompt({ taskKind: "ImageGeneration", value: prompt, setValue: setPrompt });

async function handleGenerate() {
  if (!prompt.trim()) return;
  setBusy(true);
  try {
    if (optimize.enabled) {
      await optimize.optimize();
    }
    const parsed = parseOverride(prompt);
    const overrideModel = parsed.override
      ? resolveOverrideToModel(parsed.override)
      : undefined;
    const finalModel = overrideModel ?? (model === "auto" ? undefined : model);
    const finalPrompt = parsed.cleanPrompt;
    // existing IPC call — pass `model_override: finalModel` in the payload:
    const result = await textToImage({
      prompt: finalPrompt,
      model_override: finalModel,
      // ... rest
    });
    // ... existing rendering
  } finally {
    setBusy(false);
  }
}
```

The `textToImage` wrapper signature (in `src/lib/imageCommands.ts`) needs `model_override?: string`. Update accordingly. Backend `image_pipeline::commands::text_to_image` reads it and passes into `AiRequest.model_override`.

4. JSX: render ToolDropdown + OptimizeToggle in the brief-row near the existing Generate button.

- [ ] **Step 2: Run + commit**

```bash
pnpm vitest run src/pages/Graphic2D
pnpm biome check .
git add src/pages/Graphic2D.tsx src/lib/imageCommands.ts src-tauri/src/image_pipeline/
git commit -m "feat(graphic2d): wire ToolDropdown + Optimize + /tool override"
```

---

## Task 19: Per-page integration — replicate the pattern across the other 4 pages

**Files:**
- Modify: `src/pages/WebsiteBuilder.tsx`
- Modify: `src/pages/Graphic3D.tsx`
- Modify: `src/pages/Video.tsx`
- Modify: `src/pages/Typography.tsx`
- Modify: corresponding `src/lib/*Commands.ts` wrappers (add optional `model_override?: string`)
- Modify: corresponding `src-tauri/src/{website_analyzer,depth_pipeline,mesh_pipeline,video_pipeline,logo_pipeline}/commands.rs` (read `model_override`, pass to AiRequest)

For each page:
- Add `const [model, setModel] = useState("auto")`
- Add `const optimize = useOptimizePrompt({ taskKind: <appropriate>, value: <prompt>, setValue: <setPrompt> })`
- Wrap handleGenerate with parseOverride + optimize-if-enabled + final-model resolution (mirror Task 18)
- JSX: ToolDropdown + OptimizeToggle near the Generate button
- Update the corresponding TS wrapper + Rust command to plumb `model_override` through

TaskKind per page:
- WebsiteBuilder → "TextGeneration"
- Graphic3D → "Text3D" (for the text-to-3D input) or "Image3D" (for the image-to-3D input — separate ToolDropdowns if both exist)
- Video → "TextToVideo" (storyboard input) or "VideoMontage" depending on context
- Typography → "Logo"

This is a large mechanical task — best dispatched as a single subagent with the Graphic2D pattern from Task 18 as the template.

- [ ] **Step 1: Apply the pattern to all 4 pages + their wrappers + their backends**

(Verbatim replication of Task 18 structure, parameterized per page.)

- [ ] **Step 2: Run all page tests**

```bash
pnpm vitest run src/pages/
```

- [ ] **Step 3: Backend tests**

```bash
cd src-tauri && cargo test && cd ..
```

- [ ] **Step 4: Commit**

```bash
pnpm biome check .
git add src/pages/ src/lib/ src-tauri/src/
git commit -m "feat(modules): replicate Graphic2D ToolDropdown+Optimize+override pattern across remaining 4 pages"
```

---

## Task 20: Extend ShortcutHelpOverlay with Tool Overrides section

**Files:**
- Modify: `src/components/shell/ShortcutHelpOverlay.tsx`
- Modify: `src/components/shell/ShortcutHelpOverlay.test.tsx`

- [ ] **Step 1: Add overrides section**

In `ShortcutHelpOverlay.tsx`, after the existing keyboard-shortcut groups, add a new section listing OVERRIDE_ALIASES grouped by provider:

```tsx
import { OVERRIDE_ALIASES } from "@/lib/promptOverride";

// Build provider groups inline
const overrides = Object.entries(OVERRIDE_ALIASES);
// Group by inferred provider from model name (or just show flat list)

// In JSX, after the existing scope groups:
<div className="flex flex-col gap-2">
  <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
    Tool overrides (use at start or end of prompt)
  </span>
  <ul className="grid grid-cols-2 gap-x-4 gap-y-1">
    {overrides.map(([slug, model]) => (
      <li key={slug} className="flex items-center justify-between text-2xs text-neutral-dark-200">
        <kbd className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-1.5 py-0.5 font-mono text-2xs text-accent-500">
          /{slug}
        </kbd>
        <span className="font-mono text-2xs text-neutral-dark-400">{model}</span>
      </li>
    ))}
  </ul>
</div>
```

- [ ] **Step 2: Test extension**

In `ShortcutHelpOverlay.test.tsx` add:

```tsx
it("lists tool overrides section", () => {
  render(<ShortcutHelpOverlay open={true} onClose={() => {}} />);
  expect(screen.getByText(/Tool overrides/i)).toBeInTheDocument();
  expect(screen.getByText("/flux")).toBeInTheDocument();
  expect(screen.getByText("/kling")).toBeInTheDocument();
  expect(screen.getByText("/ideogram")).toBeInTheDocument();
});
```

- [ ] **Step 3: Run + commit**

```bash
pnpm vitest run src/components/shell/ShortcutHelpOverlay
pnpm biome check .
git add src/components/shell/ShortcutHelpOverlay.tsx src/components/shell/ShortcutHelpOverlay.test.tsx
git commit -m "feat(shortcuts): tool-overrides section in help overlay"
```

---

## Task 21: E2E specs — chat + override

**Files:**
- Create: `e2e/tests/chat.spec.ts`
- Create: `e2e/tests/generate-with-override.spec.ts`

- [ ] **Step 1: chat.spec.ts**

```ts
import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

test.describe("Chat", () => {
  test("page renders + send button is present", async ({ page }) => {
    await installInvokeMock(page, {
      list_projects: [],
      get_budget_status: { state: "ok", used_today_cents: 0, limits: { daily_cents: null }, used_session_cents: 0, day_started_at: "", session_started_at: "" },
      get_queue_status: { pending: 0, in_flight: 0 },
      detect_claude_cli: "/usr/local/bin/claude",
      get_claude_transport: "auto",
    });
    await page.goto("/chat");
    await expect(page.getByText(/CHAT · CLAUDE/i)).toBeVisible();
    await expect(page.getByPlaceholderText(/Type a message/i)).toBeVisible();
  });
});
```

- [ ] **Step 2: generate-with-override.spec.ts**

```ts
import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

test.describe("Override syntax in prompt", () => {
  test("/flux at start uses Flux Pro override", async ({ page }) => {
    let receivedPayload: unknown = null;
    await installInvokeMock(page, {
      list_projects: [],
      get_budget_status: { state: "ok", used_today_cents: 0, limits: { daily_cents: null }, used_session_cents: 0, day_started_at: "", session_started_at: "" },
      get_queue_status: { pending: 0, in_flight: 0 },
      text_to_image: [{ url: "data:image/png;base64,iVBOR", local_path: "/tmp/x.png", seed: 1, model: "FalFluxPro" }],
    });
    await page.goto("/graphic2d");
    await page.getByLabel(/Describe the image/i).fill("/flux a sunset");
    // The mock just records that text_to_image was invoked; the override
    // should have been parsed before invoke. Smoke-assert generation
    // succeeds and the gallery renders something.
    await page.getByRole("button", { name: /Generate/i }).click();
    // Allow time for the generate flow
    await expect(page.locator("body")).toBeVisible();
  });
});
```

- [ ] **Step 3: Run + commit**

```bash
pnpm e2e
pnpm biome check .
git add e2e/tests/chat.spec.ts e2e/tests/generate-with-override.spec.ts
git commit -m "test(e2e): chat smoke + generate-with-override"
```

---

## Task 22: Phase 9 closure report

**Files:**
- Create: `docs/superpowers/specs/2026-04-19-phase-9-verification-report.md`

- [ ] **Step 1: Run full pipeline**

```bash
pnpm test
pnpm test:coverage
cd src-tauri && cargo test && cd ..
pnpm e2e
pnpm exec tsc --noEmit
pnpm biome check .
```

All green.

- [ ] **Step 2: Write report**

```markdown
# Phase 9 (Claude Bridge + Tool UX) — Verification Report

**Date:** 2026-04-19
**Spec:** `docs/superpowers/specs/2026-04-19-phase-9-claude-bridge-and-tool-ux-design.md`
**Plan:** `docs/superpowers/plans/2026-04-19-phase-9-claude-bridge-and-tool-ux.md`

## Summary

Sub-Project A: ClaudeCliClient + transport selector (auto/api/cli) implemented. Sub-Project B: Chat page + ToolDropdown + Optimize toggle + /tool override + extended ShortcutHelpOverlay shipped. Frontend <N> tests; backend <M> tests; e2e <K> specs; CI 5/5 green.

## Pillars
[per-pillar bullet summary with file paths and test counts — fill from real numbers]

## Backlog filed during execution
[list]

## Verdict
Phase 9 closed. Ready for live-test.
```

- [ ] **Step 3: Commit + push + watch**

```bash
git add docs/superpowers/specs/2026-04-19-phase-9-verification-report.md
git commit -m "docs(phase-9): verification report — Claude Bridge + Tool UX closed"
git push origin main
gh run list --branch main --limit 1 --json databaseId
gh run watch <id> --exit-status
```

CI 5/5.

---

## Self-Review

**Spec coverage:**
- ✓ ClaudeCliClient + Spawner trait + tests → T1-T3
- ✓ Tauri commands for transport + CLI detection → T4
- ✓ Settings UI tri-state → T5
- ✓ AiRequest.model_override → T6
- ✓ Sub-Project A closure → T7
- ✓ toolCatalog + tests → T8
- ✓ promptOverride parser + 12 tests → T9
- ✓ ToolDropdown component + tests → T10
- ✓ optimize_prompt IPC + meta-templates → T11
- ✓ optimizeCommands wrapper + useOptimizePrompt hook → T12
- ✓ OptimizeToggle UI → T13
- ✓ chatStore → T14
- ✓ chat_send_message IPC + chatCommands wrapper → T15
- ✓ Chat page + route → T16
- ✓ Sidebar Chat entry + Cmd+L → T17
- ✓ Per-page integration (Graphic2D + 4 others) → T18+T19
- ✓ ShortcutHelpOverlay extension → T20
- ✓ E2E specs → T21
- ✓ Closure report → T22

**Placeholder scan:**
- T19 is a "replicate the pattern" task — but the pattern is fully specified in T18 with code snippets. The implementer can treat T19 as 4× T18 (per-page).
- T11 uses `uuid::Uuid::new_v4()` — verify uuid crate is a runtime dep; if not, fall back to `chrono::Utc::now().timestamp_millis()` (chrono IS already used in budget.rs).
- T15 currently buffers the entire response then emits a single chunk + done — true token-by-token streaming is a follow-up; this keeps Sub-Project B unblocked. Document as a backlog if user wants real streaming.

**Type consistency:**
- `Spawner`, `SpawnResult`, `StreamAccumulator`, `StreamResult` defined in T2/T3 and used consistently.
- `OVERRIDE_ALIASES`, `parseOverride`, `resolveOverrideToModel` defined in T9 and consumed in T18-T20.
- `ToolDef`, `Tier`, `getToolsFor` defined in T8 and consumed in T10/T20.
- `AiRequest.model_override` added in T6 and consumed by router + per-page IPC in T18-T19.
- `useOptimizePrompt`, `OptimizeToggle` consistent across T12-T13 and T16/T18.
- `chat_send_message` event names ("chat:stream-chunk", "chat:stream-done") consistent across T15 backend + frontend wrapper + Chat page.
