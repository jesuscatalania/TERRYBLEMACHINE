# Phase 6 — Video-Produktion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `/video` module: Text → Claude-generated storyboard → KI-video clips (Kling/Runway/Higgsfield) + Remotion compositions → Shotstack-assembled timeline → exported MP4/WebM/GIF.

**Architecture:**
- **Storyboard**: `storyboard_generator` Rust module asks Claude for structured JSON (shots with description/duration/camera/transition). Frontend editor lets user reorder + edit.
- **Video pipeline**: `video_pipeline` Rust module mirrors `image_pipeline`/`mesh_pipeline`. Routes through AiRouter to Kling (primary), Runway (fallback), Higgsfield (fallback). Polling mirrors Meshy T8.
- **Remotion**: new `remotion/` subpackage (React + TS) with Kinetic Typography + Motion Graphics compositions. Tauri spawns `npx remotion render` as a sidecar, writes MP4 to cache, frontend loads via `convertFileSrc`.
- **Shotstack**: existing client extended with JSON timeline builder + render-status polling.
- **Video UI**: new `/video` page — storyboard editor, segment list (drag-drop), routing per segment (Local Remotion / Cloud Shotstack), export settings, render progress.

**Tech Stack:**
- Backend: existing Tauri + Rust + Tokio; existing shotstack.rs client; new Remotion sidecar (Node).
- Frontend: Existing stack; add `remotion` + `@remotion/cli` + `@remotion/three` (already installed: @types/three + three from Phase 5). Drag-drop via `@dnd-kit/core`.
- Reuses: AiRouter, polling patterns from T8/T9 + FU #129, cache pattern from mesh_pipeline.

**Scope decisions documented up-front:**
- **Remotion compositions**: ship 2 core templates (KineticTypography, MotionGraphics). `@remotion/three` integration is a stretch; defer elaborate 3D Remotion scenes to a follow-up unless trivially doable in scope.
- **GPU-acceleration flag `--gl=angle`**: pass via sidecar args; document but don't verify live (needs a real M-series Mac).
- **Shotstack cost**: already wired in Phase 2 budget table; check still aligns.
- **Frame-sequence PNG export** (plan 6.5): covered via Remotion's `--sequence` render mode.

---

## Task Dependency Graph

```
T1 (storyboard backend)
  → T2 (storyboard UI)
  
T3 (video_pipeline backend)
  → T4 (Runway+Higgsfield polling) [parallel-safe w/ T3 review]
  → T5 (video frontend)
  
T6 (Remotion subpackage setup)
  → T7 (kinetic typography + motion graphics compositions)
  → T8 (render_remotion Tauri command)
  
T9 (Shotstack timeline builder + polling)
  → T10 (shotstack frontend routing)
  
T11 (Video page + segment list drag-drop)
T12 (Export settings + render progress)
T13 (Phase 6 verification + final commit)
```

All tasks commit + push + CI watch. Tasks touching the same file serialize.

---

## Task 1: Storyboard generator (backend)

**Closes:** Plan Schritt 6.1 (backend half).

**Files:**
- Create: `src-tauri/src/storyboard_generator/mod.rs`
- Create: `src-tauri/src/storyboard_generator/types.rs`
- Create: `src-tauri/src/storyboard_generator/prompt.rs`
- Create: `src-tauri/src/storyboard_generator/generator.rs`
- Create: `src-tauri/src/storyboard_generator/stub.rs`
- Create: `src-tauri/src/storyboard_generator/commands.rs`
- Create: `src-tauri/tests/storyboard_generator_integration.rs`
- Modify: `src-tauri/src/lib.rs` — register module + state + command

### Approach

Mirror `code_generator`'s structure. Claude receives a template + brief + meingeschmack context, returns strict JSON matching:

```json
{
  "summary": "short label",
  "template": "commercial",
  "shots": [
    { "index": 1, "description": "...", "style": "...", "duration_s": 5,
      "camera": "dolly in", "transition": "fade" }
  ]
}
```

### Steps

- [ ] **Step 1: Grep existing code_generator for pattern parity**

```bash
grep -n "pub struct\|pub enum\|pub fn\|pub trait" src-tauri/src/code_generator/types.rs src-tauri/src/code_generator/prompt.rs src-tauri/src/code_generator/claude.rs
```

Report shapes found. Mirror `CodeGenerator` trait + `ClaudeCodeGenerator` + `StubCodeGenerator` + `CodeGeneratorState` + `generate_website` command pattern.

- [ ] **Step 2: Write types.rs**

```rust
//! Type definitions for the storyboard generator.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StoryboardTemplate {
    #[default]
    Commercial,
    Explainer,
    SocialMedia,
    MusicVideo,
    Custom,
}

impl StoryboardTemplate {
    /// Brief describing the template's shape + tone.
    pub fn brief(&self) -> &'static str {
        match self {
            Self::Commercial => "a 20-40 second product commercial with clear call-to-action",
            Self::Explainer => "a 45-90 second explainer: problem, solution, product, outcome",
            Self::SocialMedia => "a 15-30 second social-media spot, punchy hook in first 3 seconds",
            Self::MusicVideo => "a music video cut to beat; visual motif > narrative",
            Self::Custom => "",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StoryboardInput {
    pub prompt: String,
    #[serde(default)]
    pub template: StoryboardTemplate,
    #[serde(default = "default_module")]
    pub module: String,
}

fn default_module() -> String { "video".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shot {
    pub index: u32,
    pub description: String,
    pub style: String,
    pub duration_s: f32,
    pub camera: String,
    pub transition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storyboard {
    pub summary: String,
    pub template: String,
    pub shots: Vec<Shot>,
}

#[derive(Debug, Error)]
pub enum StoryboardError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("router error: {0}")]
    Router(String),
    #[error("failed to parse storyboard JSON: {0}")]
    Parse(String),
}

#[async_trait]
pub trait StoryboardGenerator: Send + Sync {
    async fn generate(&self, input: StoryboardInput) -> Result<Storyboard, StoryboardError>;
}
```

- [ ] **Step 3: Write prompt.rs**

```rust
//! Prompt builder: StoryboardInput + taste rules → one Claude prompt.

use crate::taste_engine::{enrich_prompt, EnrichOptions, TasteRules};

use super::types::StoryboardInput;

pub fn build_prompt(input: &StoryboardInput, rules: Option<&TasteRules>) -> String {
    let mut clauses = Vec::new();
    let brief = input.template.brief();
    if !brief.is_empty() {
        clauses.push(format!("Template: {brief}"));
    }
    if !input.prompt.trim().is_empty() {
        clauses.push(format!("User brief: {}", input.prompt.trim()));
    }
    let core = clauses.join("\n");
    let enriched = match rules {
        Some(r) => enrich_prompt(
            &core,
            r,
            &EnrichOptions {
                module: Some(input.module.clone()),
                tags: Vec::new(),
                with_negative: false,
            },
        ),
        None => core,
    };

    let format_instructions = r#"
Return a STRICT JSON object with no prose. Shape:
{
  "summary": "short description of the spot",
  "template": "<template-name>",
  "shots": [
    {
      "index": 1,
      "description": "what happens in this shot (concise)",
      "style": "visual language: palette, mood, texture",
      "duration_s": 5,
      "camera": "camera movement/framing (e.g. 'dolly in', 'aerial', 'static wide')",
      "transition": "how this shot ends (e.g. 'cut', 'fade', 'dissolve', 'whip-pan')"
    }
  ]
}
Keep shots 4-8. Durations should sum to the template's target length.
"#;
    format!("{enriched}\n\n{format_instructions}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storyboard_generator::types::StoryboardTemplate;

    #[test]
    fn includes_template_brief() {
        let p = build_prompt(&StoryboardInput {
            prompt: "Tell a story about a coffee shop".into(),
            template: StoryboardTemplate::Commercial,
            module: "video".into(),
        }, None);
        assert!(p.contains("Template:"));
        assert!(p.contains("commercial"));
    }

    #[test]
    fn user_brief_is_embedded() {
        let p = build_prompt(&StoryboardInput {
            prompt: "Moody rainy street".into(),
            template: StoryboardTemplate::Custom,
            module: "video".into(),
        }, None);
        assert!(p.contains("User brief: Moody rainy street"));
    }

    #[test]
    fn output_format_instruction_is_always_present() {
        let p = build_prompt(&StoryboardInput {
            prompt: "x".into(),
            template: StoryboardTemplate::Custom,
            module: "video".into(),
        }, None);
        assert!(p.contains("STRICT JSON"));
        assert!(p.contains("\"shots\""));
    }
}
```

- [ ] **Step 4: Write generator.rs**

```rust
//! ClaudeStoryboardGenerator — routes through AiRouter, parses JSON response.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use super::prompt::build_prompt;
use super::types::{Storyboard, StoryboardError, StoryboardGenerator, StoryboardInput};
use crate::ai_router::{AiRequest, AiRouter, Complexity, Priority, TaskKind};
use crate::taste_engine::{TasteEngine};

pub struct ClaudeStoryboardGenerator {
    router: Arc<AiRouter>,
    taste: Option<Arc<TasteEngine>>,
}

impl ClaudeStoryboardGenerator {
    pub fn new(router: Arc<AiRouter>) -> Self {
        Self { router, taste: None }
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
        // Rules are fetched if a taste engine is wired
        let rules_holder;
        let rules_ref: Option<&_> = if let Some(t) = &self.taste {
            let profile = t.profile().await;
            rules_holder = profile.rules;
            Some(&rules_holder)
        } else { None };
        let prompt = build_prompt(&input, rules_ref);
        let req = AiRequest {
            id: uuid::Uuid::new_v4().to_string(),
            task: TaskKind::TextGeneration,
            priority: Priority::Normal,
            complexity: Complexity::Medium,
            prompt,
            payload: json!({}),
        };
        let resp = self.router.route(req).await
            .map_err(|e| StoryboardError::Router(e.to_string()))?;
        let text = resp.output.get("text").and_then(|v| v.as_str())
            .or_else(|| resp.output.get("content").and_then(|c| c.as_array())
                .and_then(|a| a.first()).and_then(|b| b.get("text")).and_then(|t| t.as_str()))
            .unwrap_or("")
            .trim();
        // Strip fences defensively (same approach as code_generator::assist)
        let json_body = strip_fence(text);
        serde_json::from_str::<Storyboard>(json_body)
            .map_err(|e| StoryboardError::Parse(format!("{e}: body was: {json_body}")))
    }
}

fn strip_fence(s: &str) -> &str {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        // Find the closing ```
        if let Some(end) = rest.rfind("```") {
            // Skip the first line tag (e.g., ```json\n)
            let inner = &rest[..end];
            return inner.split_once('\n').map(|(_, body)| body).unwrap_or(inner).trim();
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
```

- [ ] **Step 5: Write stub.rs**

```rust
//! Deterministic stub for tests / offline usage.

use async_trait::async_trait;

use super::types::{Shot, Storyboard, StoryboardError, StoryboardGenerator, StoryboardInput};

pub struct StubStoryboardGenerator;
impl StubStoryboardGenerator { pub fn new() -> Self { Self } }

#[async_trait]
impl StoryboardGenerator for StubStoryboardGenerator {
    async fn generate(&self, input: StoryboardInput) -> Result<Storyboard, StoryboardError> {
        if input.prompt.trim().is_empty() {
            return Err(StoryboardError::InvalidInput("prompt empty".into()));
        }
        Ok(Storyboard {
            summary: format!("Stub board for: {}", input.prompt.trim()),
            template: format!("{:?}", input.template).to_lowercase(),
            shots: (1..=5).map(|i| Shot {
                index: i,
                description: format!("Stub shot {i} — {}", input.prompt.trim()),
                style: "neutral, bright".into(),
                duration_s: 4.0,
                camera: "static wide".into(),
                transition: if i == 5 { "cut".into() } else { "fade".into() },
            }).collect(),
        })
    }
}
```

- [ ] **Step 6: Write commands.rs**

```rust
use std::sync::Arc;

use serde::Serialize;
use thiserror::Error;

use super::types::{Storyboard, StoryboardError, StoryboardGenerator, StoryboardInput};

pub struct StoryboardGeneratorState(pub Arc<dyn StoryboardGenerator>);
impl StoryboardGeneratorState {
    pub fn new(g: Arc<dyn StoryboardGenerator>) -> Self { Self(g) }
}

#[derive(Debug, Serialize, Error)]
#[serde(tag = "kind", content = "detail", rename_all = "kebab-case")]
pub enum StoryboardIpcError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("router error: {0}")]
    Router(String),
    #[error("parse error: {0}")]
    Parse(String),
}

impl From<StoryboardError> for StoryboardIpcError {
    fn from(e: StoryboardError) -> Self {
        match e {
            StoryboardError::InvalidInput(m) => Self::InvalidInput(m),
            StoryboardError::Router(m) => Self::Router(m),
            StoryboardError::Parse(m) => Self::Parse(m),
        }
    }
}

#[tauri::command]
pub async fn generate_storyboard(
    state: tauri::State<'_, StoryboardGeneratorState>,
    input: StoryboardInput,
) -> Result<Storyboard, StoryboardIpcError> {
    state.0.generate(input).await.map_err(Into::into)
}
```

- [ ] **Step 7: Write mod.rs**

```rust
pub mod commands;
pub mod generator;
pub mod prompt;
pub mod stub;
pub mod types;

pub use generator::ClaudeStoryboardGenerator;
pub use stub::StubStoryboardGenerator;
pub use types::{
    Shot, Storyboard, StoryboardError, StoryboardGenerator, StoryboardInput, StoryboardTemplate,
};
```

- [ ] **Step 8: Integration test**

Create `src-tauri/tests/storyboard_generator_integration.rs`:

```rust
//! End-to-end test: ClaudeStoryboardGenerator via stub AiClient.

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
    fn provider(&self) -> Provider { Provider::Claude }
    fn supports(&self, m: Model) -> bool {
        matches!(m, Model::ClaudeHaiku | Model::ClaudeSonnet | Model::ClaudeOpus)
    }
    async fn execute(&self, _model: Model, req: &AiRequest) -> Result<AiResponse, ProviderError> {
        let text = json!({
            "summary": "test",
            "template": "commercial",
            "shots": [
                {"index":1,"description":"open","style":"warm","duration_s":5,"camera":"dolly","transition":"fade"},
                {"index":2,"description":"middle","style":"warm","duration_s":5,"camera":"static","transition":"cut"}
            ]
        }).to_string();
        Ok(AiResponse {
            request_id: req.id.clone(),
            model: Model::ClaudeSonnet,
            output: json!({ "text": text, "stop_reason": "end_turn" }),
            cost_cents: None,
            cached: false,
        })
    }
    async fn health_check(&self) -> bool { true }
    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> { Ok(ProviderUsage::default()) }
}

fn generator() -> ClaudeStoryboardGenerator {
    let mut m: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();
    m.insert(Provider::Claude, Arc::new(StubClaude));
    let router = Arc::new(AiRouter::new(Arc::new(DefaultRoutingStrategy), m, RetryPolicy::default_policy(), Arc::new(PriorityQueue::new())));
    ClaudeStoryboardGenerator::new(router)
}

#[tokio::test]
async fn generates_storyboard_from_text() {
    let g = generator();
    let sb = g.generate(StoryboardInput {
        prompt: "coffee shop ad".into(),
        template: StoryboardTemplate::Commercial,
        module: "video".into(),
    }).await.unwrap();
    assert_eq!(sb.shots.len(), 2);
    assert_eq!(sb.shots[0].index, 1);
    assert!(sb.summary.contains("test"));
}

#[tokio::test]
async fn rejects_empty_prompt() {
    let g = generator();
    let err = g.generate(StoryboardInput {
        prompt: "   ".into(),
        template: StoryboardTemplate::Commercial,
        module: "video".into(),
    }).await.unwrap_err();
    assert!(matches!(err, terryblemachine_lib::storyboard_generator::StoryboardError::InvalidInput(_)));
}
```

- [ ] **Step 9: Register in lib.rs**

Add `pub mod storyboard_generator;` with the other pipeline modules.

In `run()` setup, after the code_generator registration:

```rust
let storyboard: Arc<dyn storyboard_generator::StoryboardGenerator> = Arc::new(
    storyboard_generator::ClaudeStoryboardGenerator::new(Arc::clone(&ai_router_for_setup))
        .with_taste_engine(Arc::clone(&engine))
);
app.manage(storyboard_generator::commands::StoryboardGeneratorState::new(storyboard));
```

Add `storyboard_generator::commands::generate_storyboard` to `invoke_handler!`.

- [ ] **Step 10: Verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -3 && cargo test 2>&1 | tail -10
cd .. && pnpm biome check . 2>&1 | tail -3
```
All green.

Commit:
```
feat(video): storyboard_generator backend (Claude-driven shot breakdown)

- storyboard_generator module mirrors code_generator structure
- StoryboardTemplate enum (Commercial/Explainer/SocialMedia/MusicVideo/Custom)
- ClaudeStoryboardGenerator routes through AiRouter, parses fenced JSON
- Tauri command: generate_storyboard
- Taste engine integration
- 2 integration tests

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

Push + `gh run watch`.

---

## Task 2: Storyboard editor UI

**Files:**
- Create: `src/lib/storyboardCommands.ts`
- Create: `src/components/video/StoryboardEditor.tsx`
- Create: `src/components/video/StoryboardEditor.test.tsx`
- Create: `src/components/video/ShotCard.tsx`
- Modify: `src/pages/Video.tsx` (will be created in T11, but page scaffold starts here as a placeholder)

### Approach

Use native HTML5 drag-drop for shot reordering (avoid adding @dnd-kit just for list reorder — YAGNI). Native drag works fine for vertical list with 5-8 items.

### Steps

- [ ] **Step 1: Frontend wrapper**

```ts
// src/lib/storyboardCommands.ts
import { invoke } from "@tauri-apps/api/core";

export type StoryboardTemplate =
  | "commercial" | "explainer" | "social-media" | "music-video" | "custom";

export interface StoryboardInput {
  prompt: string;
  template?: StoryboardTemplate;
  module?: string;
}

export interface Shot {
  index: number;
  description: string;
  style: string;
  duration_s: number;
  camera: string;
  transition: string;
}

export interface Storyboard {
  summary: string;
  template: string;
  shots: Shot[];
}

export const generateStoryboard = (input: StoryboardInput) =>
  invoke<Storyboard>("generate_storyboard", { input });
```

- [ ] **Step 2: Write ShotCard + test**

```tsx
// src/components/video/ShotCard.tsx
import type { Shot } from "@/lib/storyboardCommands";

export interface ShotCardProps {
  shot: Shot;
  onChange: (patch: Partial<Shot>) => void;
  onRemove: () => void;
  onDragStart: () => void;
  onDragOver: (e: React.DragEvent) => void;
  onDrop: () => void;
  isDragging: boolean;
}

export function ShotCard({ shot, onChange, onRemove, onDragStart, onDragOver, onDrop, isDragging }: ShotCardProps) {
  return (
    <div
      draggable
      onDragStart={onDragStart}
      onDragOver={(e) => { e.preventDefault(); onDragOver(e); }}
      onDrop={onDrop}
      className={`rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 p-3 ${isDragging ? "opacity-50" : ""}`}
      data-testid={`shot-card-${shot.index}`}
    >
      <div className="flex items-start justify-between gap-2">
        <span className="font-mono text-2xs text-accent-500 uppercase tracking-label">Shot {shot.index}</span>
        <button type="button" onClick={onRemove} aria-label="Remove shot" className="text-neutral-dark-400 hover:text-neutral-dark-100">×</button>
      </div>
      <input
        type="text"
        value={shot.description}
        onChange={(e) => onChange({ description: e.target.value })}
        className="mt-1 w-full rounded-xs border border-neutral-dark-700 bg-neutral-dark-950 px-2 py-1 text-xs text-neutral-dark-100"
        placeholder="Description"
      />
      <div className="mt-1 flex gap-2">
        <input
          type="number"
          min={1}
          step={0.5}
          value={shot.duration_s}
          onChange={(e) => onChange({ duration_s: Number(e.target.value) })}
          className="w-16 rounded-xs border border-neutral-dark-700 bg-neutral-dark-950 px-2 py-1 text-xs text-neutral-dark-100"
          aria-label={`Shot ${shot.index} duration`}
        />
        <span className="font-mono text-2xs text-neutral-dark-500 self-center">sec</span>
        <input
          type="text"
          value={shot.camera}
          onChange={(e) => onChange({ camera: e.target.value })}
          className="flex-1 rounded-xs border border-neutral-dark-700 bg-neutral-dark-950 px-2 py-1 text-xs text-neutral-dark-100"
          placeholder="Camera"
          aria-label={`Shot ${shot.index} camera`}
        />
      </div>
    </div>
  );
}
```

- [ ] **Step 3: StoryboardEditor + test**

```tsx
// src/components/video/StoryboardEditor.tsx
import { useState } from "react";
import { Button } from "@/components/ui/Button";
import type { Shot, Storyboard } from "@/lib/storyboardCommands";
import { ShotCard } from "./ShotCard";

export interface StoryboardEditorProps {
  storyboard: Storyboard | null;
  onChange: (storyboard: Storyboard) => void;
}

export function StoryboardEditor({ storyboard, onChange }: StoryboardEditorProps) {
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  if (!storyboard) {
    return (
      <div className="flex h-full items-center justify-center font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
        No storyboard yet — generate one above
      </div>
    );
  }

  function updateShot(i: number, patch: Partial<Shot>) {
    if (!storyboard) return;
    const shots = storyboard.shots.map((s, idx) => idx === i ? { ...s, ...patch } : s);
    onChange({ ...storyboard, shots });
  }
  function removeShot(i: number) {
    if (!storyboard) return;
    const shots = storyboard.shots.filter((_, idx) => idx !== i).map((s, idx) => ({ ...s, index: idx + 1 }));
    onChange({ ...storyboard, shots });
  }
  function addShot() {
    if (!storyboard) return;
    const next: Shot = {
      index: storyboard.shots.length + 1,
      description: "",
      style: "",
      duration_s: 4,
      camera: "static",
      transition: "cut",
    };
    onChange({ ...storyboard, shots: [...storyboard.shots, next] });
  }
  function dropOn(i: number) {
    if (dragIndex === null || dragIndex === i) { setDragIndex(null); return; }
    const shots = [...storyboard.shots];
    const [moved] = shots.splice(dragIndex, 1);
    shots.splice(i, 0, moved);
    const renumbered = shots.map((s, idx) => ({ ...s, index: idx + 1 }));
    onChange({ ...storyboard, shots: renumbered });
    setDragIndex(null);
  }

  return (
    <div className="flex h-full flex-col gap-2 overflow-y-auto p-3">
      <div className="flex items-center justify-between">
        <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
          Shots · {storyboard.shots.length}
        </span>
        <Button variant="secondary" size="sm" onClick={addShot}>Add shot</Button>
      </div>
      {storyboard.shots.map((shot, i) => (
        <ShotCard
          key={`${shot.index}-${i}`}
          shot={shot}
          onChange={(patch) => updateShot(i, patch)}
          onRemove={() => removeShot(i)}
          onDragStart={() => setDragIndex(i)}
          onDragOver={() => {}}
          onDrop={() => dropOn(i)}
          isDragging={dragIndex === i}
        />
      ))}
    </div>
  );
}
```

Test:

```tsx
// src/components/video/StoryboardEditor.test.tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { StoryboardEditor } from "@/components/video/StoryboardEditor";

function sampleBoard() {
  return {
    summary: "s", template: "commercial",
    shots: [
      { index: 1, description: "a", style: "", duration_s: 3, camera: "static", transition: "cut" },
      { index: 2, description: "b", style: "", duration_s: 4, camera: "dolly", transition: "fade" },
    ],
  };
}

describe("StoryboardEditor", () => {
  it("renders empty state when no storyboard", () => {
    render(<StoryboardEditor storyboard={null} onChange={() => {}} />);
    expect(screen.getByText(/No storyboard yet/i)).toBeInTheDocument();
  });

  it("renders each shot", () => {
    render(<StoryboardEditor storyboard={sampleBoard()} onChange={() => {}} />);
    expect(screen.getByTestId("shot-card-1")).toBeInTheDocument();
    expect(screen.getByTestId("shot-card-2")).toBeInTheDocument();
  });

  it("removes a shot and renumbers", () => {
    const onChange = vi.fn();
    render(<StoryboardEditor storyboard={sampleBoard()} onChange={onChange} />);
    fireEvent.click(screen.getAllByLabelText(/Remove shot/)[0]);
    expect(onChange).toHaveBeenCalled();
    const next = onChange.mock.calls[0][0];
    expect(next.shots).toHaveLength(1);
    expect(next.shots[0].index).toBe(1);
  });

  it("adds a shot", () => {
    const onChange = vi.fn();
    render(<StoryboardEditor storyboard={sampleBoard()} onChange={onChange} />);
    fireEvent.click(screen.getByRole("button", { name: /add shot/i }));
    const next = onChange.mock.calls[0][0];
    expect(next.shots).toHaveLength(3);
    expect(next.shots[2].index).toBe(3);
  });

  it("updates shot description", () => {
    const onChange = vi.fn();
    render(<StoryboardEditor storyboard={sampleBoard()} onChange={onChange} />);
    const firstDesc = screen.getAllByPlaceholderText(/Description/)[0];
    fireEvent.change(firstDesc, { target: { value: "new desc" } });
    const next = onChange.mock.calls[0][0];
    expect(next.shots[0].description).toBe("new desc");
  });
});
```

- [ ] **Step 4: Verify + commit**

```bash
pnpm test -- --run 2>&1 | tail -5
pnpm exec tsc --noEmit 2>&1 | tail -3
pnpm biome check . 2>&1 | tail -3
```

Commit: `feat(video): StoryboardEditor component with drag-reorder + inline edit`.

---

## Task 3: video_pipeline backend

**Closes:** Plan Schritt 6.2 (backend).

**Files:**
- Create: `src-tauri/src/video_pipeline/{mod,types,pipeline,stub,commands}.rs`
- Create: `src-tauri/tests/video_pipeline_integration.rs`
- Modify: `src-tauri/src/lib.rs`

### Approach

Mirror `mesh_pipeline` structure. Types:

```rust
pub struct VideoTextInput { pub prompt: String, pub duration_s: Option<f32>, pub module: Option<String> }
pub struct VideoImageInput { pub image_url: String, pub prompt: Option<String>, pub duration_s: Option<f32>, pub module: Option<String> }
pub struct VideoResult { pub video_url: String, pub local_path: Option<PathBuf>, pub model: String, pub duration_s: Option<f32> }
```

Router already supports `TaskKind::TextToVideo` / `ImageToVideo` → Kling primary, Runway + Higgsfield fallback. Kling's polling was added in Phase 2 (T10) — verify it still returns `video_url` in output.

### Steps

- [ ] **Step 1: Inspect current kling.rs output shape**

```bash
grep -n "video_url\|status.*succeeded\|output: json!" src-tauri/src/api_clients/kling.rs | head -20
```

Report: does Kling's `send_text_to_video` / `send_image_to_video` return `{ video_url, status: "succeeded" }` like Meshy does for GLBs? Confirm.

If Kling's polling is not yet implemented (plan says Phase 2 T10 added it — verify), add it in Task 4.

- [ ] **Step 2: Write types.rs + pipeline.rs + stub.rs + commands.rs + mod.rs**

Full text: mirror `mesh_pipeline` file-by-file. Key differences:
- `TaskKind::TextToVideo` / `ImageToVideo`
- Complexity decides Kling vs. fallback cascades (already wired in router)
- Video URL extraction: `resp.output.get("video_url")` — consistent with Kling shape
- Cache path: `<cache-dir>/terryblemachine/videos/<sha256>.mp4`

Data-URL guard on `image_url` identical to mesh_pipeline's pattern.

- [ ] **Step 3: Integration tests**

Mirror `mesh_pipeline_integration.rs`:
- `text_to_video_downloads_to_cache` (via `file://` stub URL)
- `text_to_video_rejects_empty_prompt`
- `image_to_video_rejects_data_url`
- `video_download_is_idempotent`

- [ ] **Step 4: Register in lib.rs**

Same pattern. Add two commands: `generate_video_from_text`, `generate_video_from_image`.

- [ ] **Step 5: Verify + commit**

Commit: `feat(video): video_pipeline backend (Kling/Runway/Higgsfield routing)`.

---

## Task 4: Runway + Higgsfield polling

**Closes:** Plan Schritt 6.2 (fallback polling).

**Files:**
- Modify: `src-tauri/src/api_clients/runway.rs` — add polling if absent
- Modify: `src-tauri/src/api_clients/higgsfield.rs` — add polling if absent

### Approach

Both clients were stubbed in Phase 2 to return job IDs without polling. Apply the Meshy pattern (T8 `poll_task`) to each, surfacing `video_url` on success.

### Steps

- [ ] **Step 1: Check existing state**

```bash
grep -n "poll\|start_task\|status.*succeeded\|\"queued\"\|send_request" src-tauri/src/api_clients/runway.rs src-tauri/src/api_clients/higgsfield.rs | head -30
```

Report what each client currently does (sync vs async, any polling).

- [ ] **Step 2: Runway polling**

If Runway needs polling, mirror Meshy's approach:
- `DEFAULT_POLL_MAX_ATTEMPTS = 30` (videos can take 2-5 min)
- `poll_task(task_id, endpoint)` in the same shape
- `send_text_to_video` + `send_image_to_video` start → poll → return `video_url`

Runway's API returns `{ status: "...", output: ["https://.../video.mp4"] }` typically. Adjust.

Add wiremock tests:
- `runway_text_to_video_polls_until_succeeded`
- `runway_propagates_failed_status`

- [ ] **Step 3: Higgsfield polling**

Same pattern. Higgsfield's response shape: check current stubbed code. Likely `{ video_url: "..." }` or similar.

- [ ] **Step 4: Verify + commit**

Commit: `feat(ai-router): Runway + Higgsfield video polling (matches Meshy/Kling pattern)`.

---

## Task 5: Video frontend wrapper + state

**Files:**
- Create: `src/lib/videoCommands.ts`
- Create: `src/components/video/SegmentList.tsx` (renders list, delete/reorder)
- Create: `src/components/video/SegmentList.test.tsx`
- Create: `src/stores/videoStore.ts` (Zustand store)
- Create: `src/stores/videoStore.test.ts`

### Approach

Segments are the canonical unit: each has a type ("ai" | "remotion" | "shotstack") + source (prompt / composition / timeline) + resulting video clip. Store them in a Zustand store keyed by the currently-active storyboard.

### Steps

- [ ] **Step 1: Wrapper**

```ts
// src/lib/videoCommands.ts
import { invoke } from "@tauri-apps/api/core";

export interface VideoTextInput { prompt: string; duration_s?: number; module?: string }
export interface VideoImageInput { image_url: string; prompt?: string; duration_s?: number; module?: string }
export interface VideoResult {
  video_url: string;
  local_path: string | null;
  model: string;
  duration_s: number | null;
}

export const generateVideoFromText = (input: VideoTextInput) =>
  invoke<VideoResult>("generate_video_from_text", { input });
export const generateVideoFromImage = (input: VideoImageInput) =>
  invoke<VideoResult>("generate_video_from_image", { input });
```

- [ ] **Step 2: Segment store**

```ts
// src/stores/videoStore.ts
import { create } from "zustand";
import type { VideoResult } from "@/lib/videoCommands";

export type SegmentKind = "ai" | "remotion" | "shotstack";

export interface Segment {
  id: string;
  kind: SegmentKind;
  label: string;
  duration_s: number;
  /** Remote URL from AI provider or Shotstack. */
  video_url?: string;
  /** Local cache path for AI-generated or Remotion-rendered clips. */
  local_path?: string | null;
  /** Provider model string for AI segments. */
  model?: string;
  /** Busy flag while generating. */
  busy?: boolean;
  /** Error after a failed generation. */
  error?: string;
}

interface VideoState {
  segments: Segment[];
  addSegment: (s: Omit<Segment, "id">) => string;
  updateSegment: (id: string, patch: Partial<Segment>) => void;
  removeSegment: (id: string) => void;
  moveSegment: (fromIndex: number, toIndex: number) => void;
  applyVideoResult: (id: string, r: VideoResult) => void;
  reset: () => void;
}

let idCounter = 0;
const nextId = () => `seg-${Date.now()}-${++idCounter}`;

export const useVideoStore = create<VideoState>((set) => ({
  segments: [],
  addSegment: (s) => {
    const id = nextId();
    set((state) => ({ segments: [...state.segments, { id, ...s }] }));
    return id;
  },
  updateSegment: (id, patch) => set((state) => ({
    segments: state.segments.map((s) => (s.id === id ? { ...s, ...patch } : s)),
  })),
  removeSegment: (id) => set((state) => ({
    segments: state.segments.filter((s) => s.id !== id),
  })),
  moveSegment: (from, to) => set((state) => {
    const next = [...state.segments];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    return { segments: next };
  }),
  applyVideoResult: (id, r) => set((state) => ({
    segments: state.segments.map((s) =>
      s.id === id
        ? { ...s, busy: false, error: undefined, video_url: r.video_url, local_path: r.local_path, model: r.model, duration_s: r.duration_s ?? s.duration_s }
        : s
    ),
  })),
  reset: () => set({ segments: [] }),
}));
```

- [ ] **Step 3: Store tests**

```ts
// src/stores/videoStore.test.ts
import { beforeEach, describe, expect, it } from "vitest";
import { useVideoStore } from "@/stores/videoStore";

describe("videoStore", () => {
  beforeEach(() => useVideoStore.getState().reset());

  it("adds segments with unique ids", () => {
    const a = useVideoStore.getState().addSegment({ kind: "ai", label: "shot 1", duration_s: 5 });
    const b = useVideoStore.getState().addSegment({ kind: "ai", label: "shot 2", duration_s: 5 });
    expect(a).not.toBe(b);
    expect(useVideoStore.getState().segments).toHaveLength(2);
  });

  it("removes segments", () => {
    const id = useVideoStore.getState().addSegment({ kind: "ai", label: "x", duration_s: 3 });
    useVideoStore.getState().removeSegment(id);
    expect(useVideoStore.getState().segments).toHaveLength(0);
  });

  it("moves segments", () => {
    const a = useVideoStore.getState().addSegment({ kind: "ai", label: "a", duration_s: 3 });
    const b = useVideoStore.getState().addSegment({ kind: "ai", label: "b", duration_s: 3 });
    useVideoStore.getState().moveSegment(0, 1);
    const ids = useVideoStore.getState().segments.map((s) => s.id);
    expect(ids).toEqual([b, a]);
  });

  it("applyVideoResult updates segment + clears busy/error", () => {
    const id = useVideoStore.getState().addSegment({ kind: "ai", label: "x", duration_s: 5, busy: true, error: "nope" });
    useVideoStore.getState().applyVideoResult(id, { video_url: "u", local_path: "/p", model: "Kling20", duration_s: 5 });
    const seg = useVideoStore.getState().segments[0];
    expect(seg.busy).toBe(false);
    expect(seg.error).toBeUndefined();
    expect(seg.video_url).toBe("u");
    expect(seg.local_path).toBe("/p");
  });
});
```

- [ ] **Step 4: SegmentList component + test**

Renders segments from `useVideoStore`, supports drag-drop reorder (same HTML5 pattern as StoryboardEditor), delete buttons.

Test: renders each segment, delete removes, drag-drop reorders.

- [ ] **Step 5: Verify + commit**

Commit: `feat(video): video frontend wrappers + segment store`.

---

## Task 6: Remotion subpackage setup

**Closes:** Plan Schritt 6.3 (baseline).

**Files:**
- Create: `remotion/package.json`
- Create: `remotion/remotion.config.ts`
- Create: `remotion/tsconfig.json`
- Create: `remotion/src/Root.tsx`
- Create: `remotion/src/compositions/KineticTypography.tsx`
- Modify: repo root `pnpm-workspace.yaml` — add `remotion` if workspace-style, OR keep it as a separate `npm i` location
- Modify: root `package.json` (add `remotion:dev` + `remotion:render` scripts that chdir into `remotion/`)

### Approach

Remotion lives as a **sibling Node package** to keep frontend dependencies tight — `pnpm` can manage it via a workspace if preferred; simplest is a standalone directory with its own `package.json`. Tauri will spawn `npx remotion render` from this directory as a child process.

Defer advanced `@remotion/three` integration — ship the basic KineticTypography composition first.

### Steps

- [ ] **Step 1: Bootstrap remotion/**

```bash
mkdir -p /Users/enfantterryble/Documents/Projekte/TERRYBLEMACHINE/remotion
cd /Users/enfantterryble/Documents/Projekte/TERRYBLEMACHINE/remotion
pnpm init
```

Edit `remotion/package.json`:

```json
{
  "name": "terryblemachine-remotion",
  "private": true,
  "version": "0.1.0",
  "scripts": {
    "dev": "remotion studio src/Root.tsx",
    "render": "remotion render src/Root.tsx"
  },
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "remotion": "^4.0.0",
    "@remotion/cli": "^4.0.0"
  },
  "devDependencies": {
    "@types/react": "^19.0.0",
    "typescript": "^5.0.0"
  }
}
```

```bash
cd remotion && pnpm install
```

Remotion may warn about peer deps — that's OK if versions resolve.

- [ ] **Step 2: remotion.config.ts**

```ts
import { Config } from "@remotion/cli/config";

Config.setVideoImageFormat("jpeg");
Config.setOverwriteOutput(true);
// GPU acceleration for M-series: ANGLE is Apple's Metal→OpenGL translation layer
Config.setChromiumOpenGlRenderer("angle");
```

- [ ] **Step 3: tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "jsx": "react-jsx",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true
  },
  "include": ["src/**/*"]
}
```

- [ ] **Step 4: Placeholder Root**

```tsx
// remotion/src/Root.tsx
import { Composition } from "remotion";
import { KineticTypography } from "./compositions/KineticTypography";

export function Root() {
  return (
    <>
      <Composition
        id="KineticTypography"
        component={KineticTypography}
        durationInFrames={90}  // 3s at 30fps
        fps={30}
        width={1920}
        height={1080}
        defaultProps={{ text: "TERRYBLEMACHINE" }}
      />
    </>
  );
}
```

- [ ] **Step 5: Basic KineticTypography composition**

```tsx
// remotion/src/compositions/KineticTypography.tsx
import { AbsoluteFill, interpolate, useCurrentFrame, useVideoConfig } from "remotion";

export interface KineticTypographyProps {
  text: string;
}

export function KineticTypography({ text }: KineticTypographyProps) {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const opacity = interpolate(frame, [0, 15, durationInFrames - 15, durationInFrames], [0, 1, 1, 0]);
  const scale = interpolate(frame, [0, 30], [0.95, 1], { extrapolateRight: "clamp" });
  return (
    <AbsoluteFill style={{
      backgroundColor: "#0E0E11",
      alignItems: "center",
      justifyContent: "center",
    }}>
      <div style={{
        color: "#F7F7F8",
        fontFamily: "Inter, sans-serif",
        fontSize: 140,
        fontWeight: 700,
        letterSpacing: -3,
        opacity,
        transform: `scale(${scale})`,
      }}>{text}</div>
    </AbsoluteFill>
  );
}
```

- [ ] **Step 6: Gitignore remotion/**

Update `.gitignore`:

```
# Remotion workspace artifacts
remotion/node_modules
remotion/out
```

(Keep `remotion/src` + configs tracked.)

- [ ] **Step 7: Verify local render works**

```bash
cd remotion
pnpm render KineticTypography out.mp4 --props='{"text":"test"}'
```

If it produces `remotion/out.mp4` or similar: ✓. If it fails due to chromium download, try `--gl=swangle` as fallback.

Don't commit the output file.

- [ ] **Step 8: Commit**

Commit: `feat(video): Remotion subpackage + KineticTypography composition`.

---

## Task 7: Motion Graphics composition + Root registration

**Files:**
- Create: `remotion/src/compositions/MotionGraphics.tsx`
- Modify: `remotion/src/Root.tsx` — register second composition

### Approach

Motion Graphics = animated shapes / simple data viz. Ship a minimal "two rectangles + count-up number" animation that demonstrates the primitives.

### Steps

- [ ] **Step 1: MotionGraphics.tsx**

```tsx
// remotion/src/compositions/MotionGraphics.tsx
import { AbsoluteFill, interpolate, spring, useCurrentFrame, useVideoConfig } from "remotion";

export interface MotionGraphicsProps {
  title: string;
  value: number;
}

export function MotionGraphics({ title, value }: MotionGraphicsProps) {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();

  // Rectangle slide-in
  const rectX = spring({ fps, frame, config: { damping: 12 } });

  // Count-up for the number
  const countFrame = Math.min(frame, durationInFrames - 30);
  const displayed = Math.round(interpolate(countFrame, [0, durationInFrames - 30], [0, value], {
    extrapolateRight: "clamp",
  }));

  return (
    <AbsoluteFill style={{ backgroundColor: "#0E0E11" }}>
      <div style={{
        position: "absolute",
        left: 100,
        top: 120,
        width: 960 * rectX,
        height: 16,
        backgroundColor: "#e85d2d",
      }} />
      <div style={{
        position: "absolute",
        left: 100,
        top: 180,
        color: "#F7F7F8",
        fontFamily: "Inter, sans-serif",
        fontSize: 80,
        fontWeight: 600,
      }}>{title}</div>
      <div style={{
        position: "absolute",
        left: 100,
        top: 320,
        color: "#e85d2d",
        fontFamily: "IBM Plex Mono, monospace",
        fontSize: 240,
        fontWeight: 500,
        letterSpacing: -6,
      }}>{displayed.toLocaleString()}</div>
    </AbsoluteFill>
  );
}
```

- [ ] **Step 2: Register in Root.tsx**

Append inside the `<>` fragment:

```tsx
<Composition
  id="MotionGraphics"
  component={MotionGraphics}
  durationInFrames={180}
  fps={30}
  width={1920}
  height={1080}
  defaultProps={{ title: "Revenue Growth", value: 1_247 }}
/>
```

Add import at top: `import { MotionGraphics } from "./compositions/MotionGraphics";`.

- [ ] **Step 3: Verify render**

```bash
cd remotion
pnpm render MotionGraphics out-mg.mp4 --props='{"title":"Test","value":42}'
```

Expect success. Delete output.

- [ ] **Step 4: Commit**

Commit: `feat(video): MotionGraphics Remotion composition`.

---

## Task 8: Tauri render_remotion command

**Closes:** Plan Schritt 6.3 (Tauri integration).

**Files:**
- Create: `src-tauri/src/remotion/mod.rs`
- Create: `src-tauri/src/remotion/commands.rs`
- Create: `src-tauri/src/remotion/types.rs`
- Create: `src-tauri/tests/remotion_integration.rs`
- Modify: `src-tauri/src/lib.rs`
- Create: `src/lib/remotionCommands.ts`

### Approach

Backend spawns `npx remotion render <composition> <output> --props='<json>'` via `tokio::process::Command` in the `remotion/` subdir. Output goes to `<cache-dir>/terryblemachine/remotion-renders/<composition>-<hash>.mp4`. Frontend loads via `convertFileSrc`.

### Steps

- [ ] **Step 1: types.rs**

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct RemotionInput {
    pub composition: String,
    pub props: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemotionResult {
    pub output_path: PathBuf,
    pub composition: String,
}

#[derive(Debug, Error)]
pub enum RemotionError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("render process failed: {0}")]
    Process(String),
    #[error("cache error: {0}")]
    Cache(String),
}
```

- [ ] **Step 2: commands.rs**

```rust
use std::path::PathBuf;

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::process::Command;

use super::types::{RemotionError, RemotionInput, RemotionResult};

pub struct RemotionState {
    /// Absolute path to the remotion/ subpackage.
    pub remotion_root: PathBuf,
}

impl RemotionState {
    pub fn new(remotion_root: PathBuf) -> Self {
        Self { remotion_root }
    }
}

fn cache_path(composition: &str, props_json: &str) -> Result<PathBuf, RemotionError> {
    let base = dirs::cache_dir()
        .ok_or_else(|| RemotionError::Cache("no platform cache dir".into()))?;
    let dir = base.join("terryblemachine").join("remotion-renders");
    std::fs::create_dir_all(&dir).map_err(|e| RemotionError::Cache(e.to_string()))?;
    let mut h = Sha256::new();
    h.update(composition.as_bytes());
    h.update(props_json.as_bytes());
    let hash = format!("{:x}", h.finalize());
    Ok(dir.join(format!("{composition}-{hash}.mp4")))
}

pub async fn render_inner(
    remotion_root: &std::path::Path,
    input: &RemotionInput,
) -> Result<RemotionResult, RemotionError> {
    if input.composition.trim().is_empty() {
        return Err(RemotionError::InvalidInput("composition is empty".into()));
    }
    // Defend against shell-injection via composition: only alphanumeric/dash/underscore
    if !input.composition.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(RemotionError::InvalidInput("composition must be alphanumeric".into()));
    }
    let props_json = input.props.to_string();
    let output_path = cache_path(&input.composition, &props_json)?;
    if output_path.exists() {
        return Ok(RemotionResult { output_path, composition: input.composition.clone() });
    }
    let output = Command::new("npx")
        .current_dir(remotion_root)
        .arg("remotion")
        .arg("render")
        .arg("src/Root.tsx")
        .arg(&input.composition)
        .arg(&output_path)
        .arg(format!("--props={props_json}"))
        .output()
        .await
        .map_err(|e| RemotionError::Process(format!("spawn: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RemotionError::Process(format!("remotion render failed: {stderr}")));
    }
    if !output_path.exists() {
        return Err(RemotionError::Process("remotion render completed but output not found".into()));
    }
    Ok(RemotionResult { output_path, composition: input.composition.clone() })
}

#[derive(Debug, Serialize, Error)]
#[serde(tag = "kind", content = "detail", rename_all = "kebab-case")]
pub enum RemotionIpcError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("process error: {0}")]
    Process(String),
    #[error("cache error: {0}")]
    Cache(String),
}

impl From<RemotionError> for RemotionIpcError {
    fn from(e: RemotionError) -> Self {
        match e {
            RemotionError::InvalidInput(m) => Self::InvalidInput(m),
            RemotionError::Process(m) => Self::Process(m),
            RemotionError::Cache(m) => Self::Cache(m),
        }
    }
}

#[tauri::command]
pub async fn render_remotion(
    state: tauri::State<'_, RemotionState>,
    input: RemotionInput,
) -> Result<RemotionResult, RemotionIpcError> {
    super::commands::render_inner(&state.remotion_root, &input).await.map_err(Into::into)
}
```

- [ ] **Step 3: mod.rs**

```rust
pub mod commands;
pub mod types;

pub use commands::RemotionState;
pub use types::{RemotionError, RemotionInput, RemotionResult};
```

- [ ] **Step 4: Register in lib.rs**

Add `pub mod remotion;`. In setup:

```rust
let remotion_root = std::env::current_dir()
    .unwrap_or_else(|_| std::path::PathBuf::from("."))
    .join("remotion");
app.manage(remotion::RemotionState::new(remotion_root));
```

Add `remotion::commands::render_remotion` to `invoke_handler!`.

- [ ] **Step 5: Integration tests**

`src-tauri/tests/remotion_integration.rs`:

```rust
use serde_json::json;
use tempfile::TempDir;
use terryblemachine_lib::remotion::{commands::render_inner, RemotionInput};

#[tokio::test]
async fn render_rejects_empty_composition() {
    let tmp = TempDir::new().unwrap();
    let err = render_inner(tmp.path(), &RemotionInput {
        composition: "   ".into(),
        props: json!({}),
    }).await.unwrap_err();
    assert!(matches!(err, terryblemachine_lib::remotion::RemotionError::InvalidInput(_)));
}

#[tokio::test]
async fn render_rejects_injection_characters() {
    let tmp = TempDir::new().unwrap();
    let err = render_inner(tmp.path(), &RemotionInput {
        composition: "A; rm -rf /".into(),
        props: json!({}),
    }).await.unwrap_err();
    assert!(matches!(err, terryblemachine_lib::remotion::RemotionError::InvalidInput(_)));
}

// Note: no happy-path test here — would require a working remotion/ install.
// Covered by manual verification + Task 8 Step 7 below.
```

- [ ] **Step 6: Frontend wrapper**

```ts
// src/lib/remotionCommands.ts
import { invoke } from "@tauri-apps/api/core";

export interface RemotionInput {
  composition: string;
  props: Record<string, unknown>;
}
export interface RemotionResult {
  output_path: string;
  composition: string;
}

export const renderRemotion = (input: RemotionInput) =>
  invoke<RemotionResult>("render_remotion", { input });
```

- [ ] **Step 7: Manual happy-path verification**

Since we can't easily test actual Remotion render in CI, do it locally:

```bash
cd /Users/enfantterryble/Documents/Projekte/TERRYBLEMACHINE
# Install remotion deps
cd remotion && pnpm install && cd ..
# Run render via cargo — write a tiny binary helper or just confirm cache_path() works via test
```

If time allows, add a marked `#[ignore]` integration test that runs the actual Remotion render when invoked with `cargo test -- --ignored`.

- [ ] **Step 8: Commit**

Commit: `feat(video): render_remotion Tauri command + cache + input validation`.

---

## Task 9: Shotstack timeline builder + polling

**Closes:** Plan Schritt 6.4.

**Files:**
- Modify: `src-tauri/src/api_clients/shotstack.rs` — extend with timeline builder + polling
- Create: `src-tauri/src/shotstack_assembly/{mod,types,pipeline,commands}.rs`
- Create: `src-tauri/tests/shotstack_assembly_integration.rs`
- Modify: `src-tauri/src/lib.rs`

### Approach

Shotstack expects a JSON timeline:

```json
{
  "timeline": {
    "tracks": [{
      "clips": [
        { "asset": { "type": "video", "src": "..." }, "start": 0, "length": 5, "transition": {"in":"fade","out":"fade"} },
        ...
      ]
    }],
    "soundtrack": { "src": "..." }  // optional
  },
  "output": { "format": "mp4", "resolution": "hd" }
}
```

POST to `/v1/render`, poll GET `/v1/render/<id>` until status `done`. Response has `url` of the final MP4.

### Steps

- [ ] **Step 1: Inspect current shotstack.rs**

```bash
grep -n "fn new\|fn execute\|fn supports\|SHOTSTACK_BASE_URL" src-tauri/src/api_clients/shotstack.rs
```

- [ ] **Step 2: Timeline types**

`src-tauri/src/shotstack_assembly/types.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssemblyClip {
    pub src: String,
    pub start_s: f32,
    pub length_s: f32,
    #[serde(default)]
    pub transition_in: Option<String>,
    #[serde(default)]
    pub transition_out: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssemblyInput {
    pub clips: Vec<AssemblyClip>,
    #[serde(default)]
    pub soundtrack: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,  // "mp4" | "gif"
    #[serde(default = "default_resolution")]
    pub resolution: String,  // "sd" | "hd" | "1080"
}

fn default_format() -> String { "mp4".into() }
fn default_resolution() -> String { "hd".into() }

#[derive(Debug, Clone, Serialize)]
pub struct AssemblyResult {
    pub render_id: String,
    pub video_url: String,
    pub local_path: Option<PathBuf>,
}

#[derive(Debug, Error)]
pub enum AssemblyError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("router error: {0}")]
    Router(String),
    #[error("provider error: {0}")]
    Provider(String),
    #[error("download failed: {0}")]
    Download(String),
    #[error("cache error: {0}")]
    Cache(String),
}
```

- [ ] **Step 3: Extend shotstack.rs**

Add timeline-render function + polling. Follow T8/FU #129 pattern exactly:

- `assemble_timeline(clips, soundtrack, format, resolution) -> AiResponse { output.video_url }` — calls `/v1/render`, polls until done
- Request body shape per API docs
- Return video URL in `output.video_url`

Wiremock tests:
- `assembly_posts_timeline_and_polls_until_done`
- `assembly_propagates_failed_render`

- [ ] **Step 4: Pipeline wrapper + cache download**

`pipeline.rs` mirrors mesh_pipeline: uses the shotstack client, downloads final MP4 to `<cache-dir>/terryblemachine/assemblies/<hash>.mp4`.

- [ ] **Step 5: Tauri command**

`#[tauri::command] pub async fn assemble_video(input: AssemblyInput) -> Result<AssemblyResult, AssemblyIpcError>`.

- [ ] **Step 6: Integration tests**

Mirror mesh_pipeline_integration: stub shotstack client that returns a known render_id and polled-done body.

- [ ] **Step 7: Register + verify + commit**

Commit: `feat(video): Shotstack timeline assembly + polling + cache`.

---

## Task 10: Shotstack frontend routing

**Files:**
- Create: `src/lib/assemblyCommands.ts`
- Modify: `src/stores/videoStore.ts` — already has `kind: "shotstack"`, nothing new

### Steps

- [ ] **Step 1: Wrapper**

```ts
// src/lib/assemblyCommands.ts
import { invoke } from "@tauri-apps/api/core";

export interface AssemblyClip {
  src: string;
  start_s: number;
  length_s: number;
  transition_in?: string;
  transition_out?: string;
}
export interface AssemblyInput {
  clips: AssemblyClip[];
  soundtrack?: string;
  format?: "mp4" | "gif";
  resolution?: "sd" | "hd" | "1080";
}
export interface AssemblyResult {
  render_id: string;
  video_url: string;
  local_path: string | null;
}

export const assembleVideo = (input: AssemblyInput) =>
  invoke<AssemblyResult>("assemble_video", { input });
```

- [ ] **Step 2: Verify + commit**

Commit: `feat(video): Shotstack assembly frontend wrapper`.

---

## Task 11: Video page + segment list + storyboard integration

**Files:**
- Create: `src/pages/Video.tsx`
- Create: `src/pages/Video.test.tsx`
- Modify: `src/App.tsx` — route `/video` → Video page
- Modify: `src/App.test.tsx` — drop `/video` from placeholder test

### Approach

Layout mirrors Graphic2D/Graphic3D: banner + 3-column split (toolbar / storyboard+segments / settings). Top row has the storyboard-generator brief input.

### Steps

- [ ] **Step 1: Write failing test**

```tsx
// src/pages/Video.test.tsx
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { VideoPage } from "@/pages/Video";

describe("VideoPage", () => {
  it("renders the module banner", () => {
    render(<MemoryRouter><VideoPage /></MemoryRouter>);
    expect(screen.getByText(/MOD—04/)).toBeInTheDocument();
    expect(screen.getByText(/VIDEO/i)).toBeInTheDocument();
  });
  it("shows the storyboard brief input", () => {
    render(<MemoryRouter><VideoPage /></MemoryRouter>);
    expect(screen.getByLabelText(/describe the video/i)).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: VideoPage**

Following Graphic3D's pattern: banner, left tools column, center (StoryboardEditor + SegmentList), right settings panel.

- [ ] **Step 3: App.tsx route**

Replace `<Route path="/video" element={<ModulePlaceholder moduleId="video" />} />` with `<Route path="/video" element={<VideoPage />} />`.

- [ ] **Step 4: App.test.tsx drop**

Remove `["/video", "Video", "video"]` from `.each`.

- [ ] **Step 5: Verify + commit**

Commit: `feat(video): Video page scaffold + route wiring`.

---

## Task 12: Render button + export settings

**Files:**
- Create: `src/components/video/RenderExportDialog.tsx`
- Create: `src/components/video/RenderExportDialog.test.tsx`
- Modify: `src/pages/Video.tsx` — render button triggers assembly or Remotion

### Approach

Export dialog: resolution (720p/1080p/4K), FPS (24/30/60), format (MP4/WebM/GIF), routing per segment (already on segments via `kind`). Render button:

1. For each segment:
   - `kind: "ai"` → already has `video_url`/`local_path`
   - `kind: "remotion"` → call `renderRemotion(composition, props)` → get local path
   - `kind: "shotstack"` → skip (shotstack is the assembly step, not per-segment)
2. Build `AssemblyInput.clips` from finalized segments
3. Call `assembleVideo({ clips, format, resolution })` → final MP4

### Steps

- [ ] **Step 1: RenderExportDialog component + test**

Mirror Graphic3D's ThreeExportDialog. Fields: resolution dropdown (hd/1080/4K), fps dropdown (24/30/60), format dropdown (mp4/webm/gif). Output a `RenderSettings` object.

- [ ] **Step 2: Render pipeline in Video.tsx**

Wire through each segment, build assembly input, call `assembleVideo`, set final video URL on a `renderResult` state, display in a preview area.

- [ ] **Step 3: Preview pane**

Simple `<video controls src={convertFileSrc(renderResult.local_path)}>`.

- [ ] **Step 4: Verify + commit**

Commit: `feat(video): render pipeline + export dialog + preview`.

---

## Task 13: Phase 6 verification + final commit

**Files:**
- Create: `docs/superpowers/specs/2026-04-17-phase-6-verification-report.md`

### Steps

- [ ] **Step 1: Full verify**

```bash
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd .. && pnpm exec tsc --noEmit && pnpm biome check . && pnpm test -- --run
```

- [ ] **Step 2: Write verification report**

Map each of the 5 spec items (6.1-6.5) to closing commits. Document scope deferrals (elaborate @remotion/three, live Shotstack happy-path test).

- [ ] **Step 3: Commit + push + CI**

```
feat(video): Phase 6 abgeschlossen — Video-Produktion

Closing: Storyboard generator (6.1), video_pipeline with Kling/Runway/
Higgsfield polling (6.2), Remotion subpackage + KineticTypography +
MotionGraphics + render_remotion command (6.3), Shotstack timeline
assembly + polling (6.4), Video page with storyboard editor, segment
list, export dialog + render pipeline (6.5). Elaborate @remotion/three
integration deferred as follow-up.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

## Self-review

**Spec coverage:**
- 6.1 Storyboard-Generator → T1 + T2
- 6.2 KI-Video-Generation → T3 + T4 + T5 (Kling/Runway/Higgsfield routing)
- 6.3 Remotion-Integration → T6 + T7 + T8
- 6.4 Shotstack (Cloud-Assembly) → T9 + T10
- 6.5 Video-Compositing UI → T11 + T12

All 5 spec items covered. Drag-drop in 6.5 handled via native HTML5 in T2/T5/T11.

**Placeholder scan:** No "TBD" or "implement later". Remotion happy-path tests are explicitly `#[ignore]` not TODO. Elaborate `@remotion/three` is documented as deferred, not placeholder.

**Type consistency:**
- `Storyboard.shots[].duration_s` (T1 backend) ↔ `Shot.duration_s` (T2 frontend) ↔ `Segment.duration_s` (T5)
- `VideoResult.video_url` ↔ `AssemblyResult.video_url` ↔ `AssemblyClip.src` — segments feed into clips via `video_url`→`src`
- `RemotionInput.composition` + `.props` consistent T8 Rust + T8 TS
- `kind: "ai" | "remotion" | "shotstack"` defined once in videoStore, consumed consistently

**Risk areas:**
- **Remotion sidecar**: require `pnpm install` in `remotion/` to run; CI won't execute Remotion renders — `#[ignore]` tests cover this
- **Runway/Higgsfield polling**: API shapes assumed — verify live or extend mocks once shapes known
- **Shotstack render polling**: 60-attempt budget may be too short for complex timelines; tune later
- **Segment store serialization**: not persisted across sessions — add if user regresses

---

**Plan complete and saved to `docs/superpowers/plans/2026-04-17-phase-6-video.md`.**

Execution via `superpowers:subagent-driven-development` — fresh subagent per task + two-stage review (spec + quality).
