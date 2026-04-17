# Phase 1–4 Gap Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close all 19 verified gaps from the Phase 1-4 audit so every item in `docs/ENTWICKLUNG-SCHRITT-FUER-SCHRITT.md` (lines 269-921) is honestly satisfied before starting Phase 5.

**Architecture:** Tasks ordered by dependency. Foundation tasks (client registration, pipeline wiring) first — they unblock runtime behavior for later tasks. UI/feature tasks grouped by phase. Final task is a cross-phase verification pass.

**Tech Stack:** Rust (Tauri v2, Tokio, serde, thiserror, wiremock, security-framework), TypeScript (React 19, Tailwind v3, Zustand, Framer Motion, Fabric.js, Monaco), Node (Playwright sidecar), pnpm/cargo.

**Ground rules for every task:**
- TDD: Write failing test first, watch it fail, implement, watch it pass.
- **Verification BEFORE completion claims**: every task ends with the exact verify commands + expected output.
- **Last step before `git commit` is always `pnpm biome check .` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check`** (per standing user rule).
- Commit messages follow the project convention: `feat(module): …` / `fix(module): …` / `test(module): …`.
- Every task ends with `git push origin main` and a `gh run watch` to confirm CI green.

---

## Task Dependency Graph

```
T1 (client registry)
  └─ T2 (pipeline runtime)
       └─ T3 (pipeline integration tests)
                                                   
T4, T5, T6, T7 (P1 foundation — parallelizable)
T8 (Claude Vision) → T9 (Taste live loop)
T10 (Kling I2V + Runway Motion Brush + Ideogram v3)
T11 (URL analyzer assets + UI) → T12 (Claude-Assist) → T13 (Website export dialog)
T14 (Desktop 1920 + tests)
T15 (Flux-2-Pro + fallbacks) [T1 landed]
T16 (Inpainting end-to-end) [T1, T2 landed]
T17 (Flip/Crop/Resize/Selection)
T18 (Text picker)
T19 (PDF+GIF export)
T20 (Cross-phase verification + Phase 4 honest re-commit)
```

Parallelization note: within each "Group" below, tasks can be dispatched to parallel subagents. Between groups, foundation tasks must land first.

---

## Task 1: Register AI provider clients at runtime

**Closes:** P2 #78 (blocker).

**Files:**
- Modify: `src-tauri/src/lib.rs:35-46,50-85`
- Create: `src-tauri/src/api_clients/registry.rs`
- Modify: `src-tauri/src/api_clients/mod.rs` — add `pub mod registry;`
- Test: `src-tauri/src/api_clients/registry.rs` (inline `#[cfg(test)]`)

**Approach:** Add a `build_default_clients(keystore)` helper returning `HashMap<Provider, Arc<dyn AiClient>>`. All 9 clients are instantiated eagerly — they do not hit the network until an `execute` call, so it's safe to register them even when no keychain key is set. `get_api_key` already returns `ProviderError::Auth` when a key is missing; the router then propagates that as non-retriable on same model but retriable on another → fallback chain works naturally.

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/api_clients/registry.rs`:

```rust
//! Registers all 9 provider clients with a shared [`KeyStore`]. Used by
//! `lib::run` to populate the router's client map.

use std::collections::HashMap;
use std::sync::Arc;

use crate::ai_router::{AiClient, Provider};
use crate::keychain::KeyStore;

use super::{
    claude::ClaudeClient, fal::FalClient, higgsfield::HiggsfieldClient, ideogram::IdeogramClient,
    kling::KlingClient, meshy::MeshyClient, replicate::ReplicateClient, runway::RunwayClient,
    shotstack::ShotstackClient,
};

/// Build the full set of provider clients. Clients do not hit the network
/// until dispatched; missing API keys surface as `ProviderError::Auth`
/// during `execute`, which the router treats as "try another model".
pub fn build_default_clients(
    keystore: Arc<dyn KeyStore>,
) -> HashMap<Provider, Arc<dyn AiClient>> {
    let mut m: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();
    m.insert(Provider::Claude, Arc::new(ClaudeClient::new(keystore.clone())));
    m.insert(Provider::Kling, Arc::new(KlingClient::new(keystore.clone())));
    m.insert(Provider::Runway, Arc::new(RunwayClient::new(keystore.clone())));
    m.insert(Provider::Higgsfield, Arc::new(HiggsfieldClient::new(keystore.clone())));
    m.insert(Provider::Shotstack, Arc::new(ShotstackClient::new(keystore.clone())));
    m.insert(Provider::Ideogram, Arc::new(IdeogramClient::new(keystore.clone())));
    m.insert(Provider::Meshy, Arc::new(MeshyClient::new(keystore.clone())));
    m.insert(Provider::Fal, Arc::new(FalClient::new(keystore.clone())));
    m.insert(Provider::Replicate, Arc::new(ReplicateClient::new(keystore)));
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keychain::InMemoryKeyStore;

    #[test]
    fn registry_contains_all_nine_providers() {
        let ks: Arc<dyn KeyStore> = Arc::new(InMemoryKeyStore::new());
        let m = build_default_clients(ks);
        for p in [
            Provider::Claude, Provider::Kling, Provider::Runway, Provider::Higgsfield,
            Provider::Shotstack, Provider::Ideogram, Provider::Meshy, Provider::Fal,
            Provider::Replicate,
        ] {
            assert!(m.contains_key(&p), "missing provider {p:?}");
        }
        assert_eq!(m.len(), 9);
    }
}
```

- [ ] **Step 2: Run test, confirm it fails (module not declared)**

```bash
cd src-tauri && cargo test --lib registry_contains_all_nine_providers
```
Expected: FAIL — "could not find `registry` in `api_clients`" or similar.

- [ ] **Step 3: Declare the module**

Edit `src-tauri/src/api_clients/mod.rs` — append `pub mod registry;`.

Before running test, check each client's `new(keystore)` signature matches — if any client (e.g. `FalClient::new`) takes a different arg list, adjust the registry call accordingly. Confirm the `InMemoryKeyStore` path: if `crate::keychain::InMemoryKeyStore` doesn't exist, the test must use whatever in-memory stub already exists (e.g. `keychain::testing::InMemoryKeyStore` or similar). Grep first: `rg 'pub struct.*KeyStore' src-tauri/src/keychain/`.

- [ ] **Step 4: Run test, confirm it passes**

```bash
cd src-tauri && cargo test --lib registry_contains_all_nine_providers
```
Expected: PASS.

- [ ] **Step 5: Wire registry into lib::run**

Edit `src-tauri/src/lib.rs`:

Replace `HashMap::new()` at line 43 with `api_clients::registry::build_default_clients(keystore.clone())`. Update the comment block immediately above to state: "All 9 provider clients are registered. Each resolves its API key from the keychain at execute time; missing keys surface as Auth errors handled via fallback."

- [ ] **Step 6: Run full Rust test suite**

```bash
cd src-tauri && cargo test --lib 2>&1 | tail -5
```
Expected: all tests pass, no regressions.

- [ ] **Step 7: Final verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -5
cd .. && pnpm biome check . 2>&1 | tail -5
```
Expected: clean.

```bash
git add src-tauri/src/api_clients/registry.rs src-tauri/src/api_clients/mod.rs src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
fix(ai-router): Register all 9 provider clients at runtime

lib::run übergab bisher HashMap::new() an den AiRouter, wodurch jeder
route_request-Call NoClient zurückgab. Neuer api_clients::registry::
build_default_clients(keystore) instanziiert alle 9 Clients eager;
fehlende Keychain-Keys surfacen als ProviderError::Auth → Fallback-Chain
greift wie vorgesehen.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 2: Bind RouterImagePipeline in Tauri runtime

**Closes:** P4 #86 (blocker). **Depends on:** Task 1.

**Files:**
- Modify: `src-tauri/src/lib.rs:81-83`

- [ ] **Step 1: Replace StubImagePipeline with RouterImagePipeline in setup()**

Edit `src-tauri/src/lib.rs`. Replace the block starting at line 81 (the comment "// Image pipeline — stub until AiClient provider keys arrive." and the `StubImagePipeline::new()` registration) with:

```rust
            // Image pipeline — routed through the production AiRouter with
            // taste-engine enrichment. Missing provider keys bubble up as
            // routing errors rather than stub URLs.
            let pipeline: Arc<dyn ImagePipeline> = Arc::new(
                image_pipeline::RouterImagePipeline::new(
                    ai_router.clone(),
                    Some(Arc::clone(&engine)),
                ),
            );
            app.manage(ImagePipelineState::new(pipeline));
```

Adjust the `RouterImagePipeline::new` signature check first with `grep "pub fn new" src-tauri/src/image_pipeline/pipeline.rs`. If the signature takes `(Arc<AiRouter>, Arc<TasteEngine>)` without Option, drop the `Some()` wrap and pass `Arc::clone(&engine)` directly.

Import update at top of `lib.rs`:

```rust
use image_pipeline::{ImagePipeline, RouterImagePipeline};
```

Remove the `StubImagePipeline` import (no longer used in runtime — but keep the re-export in `image_pipeline/mod.rs` for test use).

- [ ] **Step 2: Confirm build still compiles and full Rust tests pass**

```bash
cd src-tauri && cargo test --lib 2>&1 | tail -5
```
Expected: PASS.

- [ ] **Step 3: Final verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -5
cd .. && pnpm biome check . 2>&1 | tail -5
```
Expected: clean.

```bash
git add src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
fix(graphic2d): Wire RouterImagePipeline into Tauri runtime

Ersetzt StubImagePipeline durch die produktive Pipeline mit AiRouter +
TasteEngine. Frontend-Image-Generation geht jetzt durch echte Routing-
Logik; fehlende Keys → AllFallbacksFailed (statt stub:// URLs).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 3: Integration tests for RouterImagePipeline

**Closes:** P4 #87.

**Files:**
- Create: `src-tauri/tests/image_pipeline_integration.rs`

**Approach:** Stand up a fake `AiClient` (`StubFalClient`) that returns canned fal.ai-shaped JSON responses, build an `AiRouter` with only that client, wrap in `RouterImagePipeline`, exercise all four methods (text_to_image, image_to_image, upscale, variants). Tests live under `src-tauri/tests/` so they're proper integration tests (compile against the public crate API).

- [ ] **Step 1: Write the test file**

Create `src-tauri/tests/image_pipeline_integration.rs`:

```rust
//! End-to-end tests for RouterImagePipeline with a stubbed provider.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use terryblemachine_lib::ai_router::{
    AiClient, AiRequest, AiResponse, AiRouter, DefaultRoutingStrategy, Model, PriorityQueue,
    Provider, ProviderError, ProviderUsage, RetryPolicy,
};
use terryblemachine_lib::image_pipeline::{
    GenerateVariantsInput, Image2ImageInput, ImagePipeline, RouterImagePipeline, Text2ImageInput,
    UpscaleInput,
};

/// Returns a fal.ai-shaped response: `{ "images": [{ "url": "...", ... }] }`.
struct StubFalClient;

#[async_trait]
impl AiClient for StubFalClient {
    async fn execute(&self, request: AiRequest) -> Result<AiResponse, ProviderError> {
        let url = format!("https://fake.fal/{}.png", request.prompt.len());
        Ok(AiResponse {
            model: Model::FalFluxPro,
            payload: json!({
                "images": [{ "url": url, "width": 1024, "height": 1024 }]
            }),
            usage: ProviderUsage::default(),
        })
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn get_usage(&self) -> Result<ProviderUsage, ProviderError> {
        Ok(ProviderUsage::default())
    }
}

fn router_with_stub() -> Arc<AiRouter> {
    let mut clients: HashMap<Provider, Arc<dyn AiClient>> = HashMap::new();
    clients.insert(Provider::Fal, Arc::new(StubFalClient));
    clients.insert(Provider::Replicate, Arc::new(StubFalClient));
    Arc::new(AiRouter::new(
        Arc::new(DefaultRoutingStrategy),
        clients,
        RetryPolicy::default_policy(),
        Arc::new(PriorityQueue::new()),
    ))
}

#[tokio::test]
async fn text_to_image_returns_fal_url() {
    let pipeline = RouterImagePipeline::new(router_with_stub(), None);
    let out = pipeline
        .text_to_image(Text2ImageInput {
            prompt: "abc".into(),
            module: "graphic2d".into(),
            complexity: None,
        })
        .await
        .expect("pipeline returns result");
    assert!(out.url.contains("fake.fal"));
    assert_eq!(out.model, Model::FalFluxPro);
}

#[tokio::test]
async fn image_to_image_passes_source_url() {
    let pipeline = RouterImagePipeline::new(router_with_stub(), None);
    let out = pipeline
        .image_to_image(Image2ImageInput {
            prompt: "style".into(),
            source_url: "https://src/a.png".into(),
            module: "graphic2d".into(),
        })
        .await
        .expect("ok");
    assert!(out.url.starts_with("https://fake.fal"));
}

#[tokio::test]
async fn upscale_returns_result() {
    let pipeline = RouterImagePipeline::new(router_with_stub(), None);
    let out = pipeline
        .upscale(UpscaleInput {
            image_url: "https://src/a.png".into(),
            scale: 2,
            module: "graphic2d".into(),
        })
        .await
        .expect("ok");
    assert_eq!(out.model, Model::FalRealEsrgan);
}

#[tokio::test]
async fn variants_yields_requested_count() {
    let pipeline = RouterImagePipeline::new(router_with_stub(), None);
    let out = pipeline
        .variants(GenerateVariantsInput {
            prompt: "logo".into(),
            count: 4,
            module: "graphic2d".into(),
        })
        .await
        .expect("ok");
    assert_eq!(out.len(), 4);
    for r in &out {
        assert!(r.url.contains("fake.fal"));
    }
}
```

Before saving, grep `RouterImagePipeline::new` and the 4 input types in `src-tauri/src/image_pipeline/types.rs` + `pipeline.rs` to confirm field names. Fix mismatches inline.

- [ ] **Step 2: Run — expect compile errors or test failures**

```bash
cd src-tauri && cargo test --test image_pipeline_integration 2>&1 | tail -40
```
Expected: either 4 passes (if RouterImagePipeline is already correct) or compile errors pointing to field-name mismatches.

- [ ] **Step 3: Fix field-name mismatches in the test until it compiles and passes**

If the pipeline or input types need minor adjustments (e.g. the `variants` method clamps count > 8 and you need to adjust test), fix the test rather than the production code unless production behavior is actually wrong.

- [ ] **Step 4: Run all tests once more**

```bash
cd src-tauri && cargo test 2>&1 | tail -10
```
Expected: all green.

- [ ] **Step 5: Final verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -5
cd .. && pnpm biome check . 2>&1 | tail -5
```

```bash
git add src-tauri/tests/image_pipeline_integration.rs
git commit -m "test(graphic2d): Integration tests for RouterImagePipeline

Vier End-to-End Tests (text_to_image/image_to_image/upscale/variants)
gegen einen StubFalClient. Schließt die TDD-Lücke aus Schritt 4.1.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 4: Sidebar collapse wiring (visual)

**Closes:** P1 #74.

**Files:**
- Modify: `src/components/shell/Shell.tsx`
- Modify: `src/components/shell/Sidebar.tsx` (compact view)
- Test: `src/components/shell/Shell.test.tsx` (create)

- [ ] **Step 1: Write the failing test**

Create `src/components/shell/Shell.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it } from "vitest";
import { Shell } from "@/components/shell/Shell";
import { useAppStore } from "@/stores/appStore";

function renderShell() {
  return render(
    <MemoryRouter>
      <Shell onNew={() => {}}>
        <div data-testid="content">content</div>
      </Shell>
    </MemoryRouter>,
  );
}

describe("Shell", () => {
  beforeEach(() => {
    useAppStore.setState({ sidebarOpen: true, theme: "dark", activeModule: "website" });
  });

  it("uses the expanded grid when sidebarOpen is true", () => {
    const { container } = renderShell();
    const grid = container.querySelector("[data-testid='shell-grid']");
    expect(grid?.className).toMatch(/grid-cols-\[15rem_1fr\]/);
  });

  it("uses the collapsed grid when sidebarOpen is false", () => {
    useAppStore.setState({ sidebarOpen: false });
    const { container } = renderShell();
    const grid = container.querySelector("[data-testid='shell-grid']");
    expect(grid?.className).toMatch(/grid-cols-\[3\.5rem_1fr\]/);
  });
});
```

- [ ] **Step 2: Run — confirm failure**

```bash
pnpm test -- --run src/components/shell/Shell.test.tsx 2>&1 | tail -15
```
Expected: FAIL (`shell-grid` not found or className mismatch).

- [ ] **Step 3: Update Shell.tsx**

Grep current Shell.tsx first. Replace the hardcoded `grid-cols-[15rem_1fr]` with a `useAppStore` read:

```tsx
import { useAppStore } from "@/stores/appStore";
// …
export function Shell({ onNew, children }: ShellProps) {
  const sidebarOpen = useAppStore((s) => s.sidebarOpen);
  const gridCols = sidebarOpen ? "grid-cols-[15rem_1fr]" : "grid-cols-[3.5rem_1fr]";
  return (
    <div data-testid="shell-grid" className={`grid h-screen ${gridCols} …existing classes`}>
      {/* … */}
    </div>
  );
}
```

Leave existing Shell.tsx classes intact — only add `data-testid="shell-grid"` and swap the fixed column class for the dynamic one.

- [ ] **Step 4: Make Sidebar compact when closed**

In `src/components/shell/Sidebar.tsx`, read `sidebarOpen`. When false, hide the module labels and show only icons. Keep the toggle button visible and labeled `Expand sidebar` / `Collapse sidebar` via `aria-label`.

- [ ] **Step 5: Run — confirm pass**

```bash
pnpm test -- --run src/components/shell/Shell.test.tsx 2>&1 | tail -10
```
Expected: 2/2 PASS.

- [ ] **Step 6: Final verify + commit**

```bash
pnpm exec tsc --noEmit && pnpm biome check . 2>&1 | tail -3
```

```bash
git add src/components/shell/Shell.tsx src/components/shell/Shell.test.tsx src/components/shell/Sidebar.tsx
git commit -m "fix(core-ui): Sidebar collapse togglet tatsächlich den Layout-Grid

sidebarOpen wird jetzt in Shell.tsx gelesen — 15rem expanded,
3.5rem collapsed. Sidebar zeigt im Collapsed-Zustand nur Icons.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 5: Recents dropdown

**Closes:** P1 #75.

**Files:**
- Create: `src/components/projects/RecentsMenu.tsx`
- Create: `src/components/projects/RecentsMenu.test.tsx`
- Modify: `src/components/shell/Header.tsx` — insert `<RecentsMenu />`

- [ ] **Step 1: Write the failing test**

```tsx
// src/components/projects/RecentsMenu.test.tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { RecentsMenu } from "@/components/projects/RecentsMenu";
import { useProjectStore } from "@/stores/projectStore";

describe("RecentsMenu", () => {
  beforeEach(() => {
    useProjectStore.setState({
      recents: [
        { id: "a", name: "Alpha", path: "/tmp/alpha", module: "website" },
        { id: "b", name: "Beta", path: "/tmp/beta", module: "graphic2d" },
      ],
      open: null,
    });
  });

  it("renders each recent entry when opened", () => {
    render(<RecentsMenu />);
    fireEvent.click(screen.getByRole("button", { name: /recent/i }));
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    expect(screen.getByText("Beta")).toBeInTheDocument();
  });

  it("shows empty state when recents is empty", () => {
    useProjectStore.setState({ recents: [] });
    render(<RecentsMenu />);
    fireEvent.click(screen.getByRole("button", { name: /recent/i }));
    expect(screen.getByText(/no recent projects/i)).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Confirm failure**

```bash
pnpm test -- --run src/components/projects/RecentsMenu.test.tsx 2>&1 | tail -10
```
Expected: FAIL (module missing).

- [ ] **Step 3: Implement RecentsMenu.tsx**

```tsx
import { Clock } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { useProjectStore } from "@/stores/projectStore";

export function RecentsMenu() {
  const [open, setOpen] = useState(false);
  const recents = useProjectStore((s) => s.recents);
  const openProject = useProjectStore((s) => s.openProject);

  return (
    <div className="relative">
      <Button
        variant="ghost"
        size="sm"
        aria-label="Recent projects"
        onClick={() => setOpen((v) => !v)}
      >
        <Clock className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
        Recent
      </Button>
      {open ? (
        <div className="absolute right-0 top-full z-20 mt-1 w-64 rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 shadow-xl">
          {recents.length === 0 ? (
            <div className="p-3 font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              No recent projects
            </div>
          ) : (
            <ul className="max-h-80 overflow-y-auto">
              {recents.map((p) => (
                <li key={p.id}>
                  <button
                    type="button"
                    className="w-full px-3 py-2 text-left hover:bg-neutral-dark-800"
                    onClick={() => {
                      openProject(p);
                      setOpen(false);
                    }}
                  >
                    <div className="text-xs text-neutral-dark-100">{p.name}</div>
                    <div className="font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
                      {p.module}
                    </div>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : null}
    </div>
  );
}
```

- [ ] **Step 4: Mount in Header.tsx**

Insert `<RecentsMenu />` near the Settings button.

- [ ] **Step 5: Confirm pass**

```bash
pnpm test -- --run src/components/projects/RecentsMenu.test.tsx 2>&1 | tail -10
```
Expected: PASS.

- [ ] **Step 6: Final verify + commit**

```bash
pnpm exec tsc --noEmit && pnpm biome check . 2>&1 | tail -3
```

```bash
git add src/components/projects/RecentsMenu.tsx src/components/projects/RecentsMenu.test.tsx src/components/shell/Header.tsx
git commit -m "feat(core-ui): Recents-Dropdown im Header

Rendert die letzten 10 Projekte aus projectStore.recents mit Klick-zu-
Öffnen. Empty-State-Hinweis wenn keine Recents vorhanden.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 6: Tauri project command tests

**Closes:** P1 #76.

**Files:**
- Create: `src-tauri/tests/projects_commands.rs`

**Approach:** Integration tests — invoke the command functions directly against a `tempfile::TempDir` root. Cover `create_project`, `open_project`, `list_projects`, `delete_project`, `projects_root`.

- [ ] **Step 1: Inspect existing command signatures**

```bash
grep -n "pub fn\|pub async fn\|#\[tauri::command\]" src-tauri/src/projects/commands.rs
```

Confirm what each command returns and what state it requires.

- [ ] **Step 2: Write integration test**

Create `src-tauri/tests/projects_commands.rs`:

```rust
//! Integration tests for the Tauri project commands.
//!
//! We bypass the Tauri runtime and call each command with its State<T>
//! equivalent directly — the `State` only wraps an `Arc<ProjectStoreState>`,
//! which is easy to construct in tests.

use std::sync::Arc;

use tempfile::TempDir;
use terryblemachine_lib::projects::commands::{ProjectStoreState};
use terryblemachine_lib::projects::{storage::Storage, ProjectInput};

fn store(root: &TempDir) -> Arc<ProjectStoreState> {
    ProjectStoreState::new(root.path().to_path_buf())
}

#[tokio::test]
async fn create_and_open_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let s = store(&tmp);
    let created = s
        .create(ProjectInput {
            name: "demo".into(),
            module: "website".into(),
            description: "".into(),
        })
        .expect("create");
    let loaded = s.open(&created.id).expect("open");
    assert_eq!(loaded.name, "demo");
}

#[tokio::test]
async fn list_returns_created_projects_newest_first() {
    let tmp = TempDir::new().unwrap();
    let s = store(&tmp);
    s.create(ProjectInput { name: "a".into(), module: "website".into(), description: "".into() })
        .unwrap();
    s.create(ProjectInput { name: "b".into(), module: "video".into(), description: "".into() })
        .unwrap();
    let list = s.list().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].name, "b");
}

#[tokio::test]
async fn delete_removes_folder() {
    let tmp = TempDir::new().unwrap();
    let s = store(&tmp);
    let p = s
        .create(ProjectInput { name: "tmp".into(), module: "website".into(), description: "".into() })
        .unwrap();
    s.delete(&p.id).unwrap();
    assert!(s.open(&p.id).is_err());
}
```

Before committing, confirm the real API: `ProjectStoreState`, `ProjectInput`, `.create/.open/.list/.delete` method names. If the actual API uses different names (e.g. `open_project`, `list_projects` free functions instead of methods), adjust the test to call them directly with `tauri::State::new(state)` stubbing where needed. Grep first:

```bash
grep -n "pub fn\|pub async fn" src-tauri/src/projects/commands.rs src-tauri/src/projects/storage.rs src-tauri/src/projects/mod.rs
```

- [ ] **Step 3: Run — fix as needed until green**

```bash
cd src-tauri && cargo test --test projects_commands 2>&1 | tail -15
```

- [ ] **Step 4: Final verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -3
cd .. && pnpm biome check . 2>&1 | tail -3
```

```bash
git add src-tauri/tests/projects_commands.rs
git commit -m "test(core-ui): Integration-Tests für Tauri-Project-Commands

Deckt create/open/list/delete als direkte Aufrufe gegen einen TempDir-
Root ab. Schließt die Test-Lücke aus Schritt 1.4-Prüfung.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 7: Undo/Redo persistence + store integration

**Closes:** P1 #77.

**Files:**
- Modify: `src/stores/historyStore.ts` — add `serialize()` / `hydrate()` helpers
- Modify: `src/stores/projectStore.ts` — stop `history.clear()` on open; load `history.json` if present, persist on close
- Modify: `src/stores/appStore.ts`, `src/stores/uiStore.ts`, `src/stores/promptHistoryStore.ts` — route mutations through `historyStore.push`
- Create: `src/stores/storeIntegration.test.ts`

**Approach:** Minimal-invasive — introduce a `withUndo<S>(store, action)` helper that wraps a store mutation with a `historyStore.push({ do, undo })`. Don't rewrite every store — just pick the user-facing mutations that should be undoable:
- `appStore.setActiveModule` — YES (navigating is undoable)
- `promptHistoryStore.push` — YES (deleting a prompt is undoable, but individual `push` entries are not — treat this as a non-undo event)
- `projectStore.openProject/closeProject` — NO (these are explicit user operations that should clear history, as they already do)
- `uiStore.notify` — NO (toasts are ephemeral)

So realistic integration: at minimum, `appStore.setActiveModule` uses the helper. Others explicitly opt out and document why.

- [ ] **Step 1: Add serialization to historyStore**

Read `src/stores/historyStore.ts` first. Add two methods:

```ts
interface HistoryStore {
  // …existing…
  serialize: () => string;        // JSON of past + future
  hydrate: (s: string) => void;   // parse + replace past/future
}
```

The stored commands must round-trip — since commands carry functions (`do`, `undo`), serialization can only capture undoable state data, not arbitrary closures. Restrict serialization to labels + metadata; on hydrate, reconstruct a limited "read-only history" list that surfaces in an Undo/Redo timeline without replay. (This matches how real editors work — you can see recent ops but old sessions aren't re-executable.) For full replay you'd need a command registry keyed by name; defer to a future task if requested.

Concretely:

```ts
type SerializableCommand = { label: string; at: number };

serialize: () => {
  const past = get().past.map((c) => ({ label: c.label, at: c.at }));
  const future = get().future.map((c) => ({ label: c.label, at: c.at }));
  return JSON.stringify({ past, future });
},
hydrate: (raw: string) => {
  try {
    const parsed = JSON.parse(raw) as { past: SerializableCommand[]; future: SerializableCommand[] };
    // Restore as read-only markers — do/undo are no-ops.
    const noop = () => {};
    set({
      past: parsed.past.map((m) => ({ label: m.label, at: m.at, do: noop, undo: noop })),
      future: parsed.future.map((m) => ({ label: m.label, at: m.at, do: noop, undo: noop })),
    });
  } catch {
    // Corrupt file — start fresh.
  }
},
```

Add `at: number` (timestamp) + `label: string` to existing `Command` type if not present.

- [ ] **Step 2: Persist history on project close, load on open**

In `projectStore.openProject`:
- Remove `historyStore.getState().clear()`.
- Attempt to read `<project.path>/history.json` via Tauri fs. If present, call `historyStore.getState().hydrate(contents)`.

In `projectStore.closeProject`:
- Before clearing, call `historyStore.getState().serialize()` and write to `<project.path>/history.json` via Tauri fs.
- Then clear.

New Tauri commands `read_project_history(project_path)` / `write_project_history(project_path, json)` in `src-tauri/src/projects/commands.rs`:

```rust
#[tauri::command]
pub fn read_project_history(path: PathBuf) -> Result<String, IpcError> {
    let file = path.join("history.json");
    if !file.exists() { return Ok("{\"past\":[],\"future\":[]}".into()); }
    std::fs::read_to_string(&file).map_err(|e| IpcError::Io(e.to_string()))
}

#[tauri::command]
pub fn write_project_history(path: PathBuf, json: String) -> Result<(), IpcError> {
    let file = path.join("history.json");
    std::fs::write(&file, json).map_err(|e| IpcError::Io(e.to_string()))
}
```

Register both in `lib.rs:generate_handler!`.

Frontend wrapper in `src/lib/projectCommands.ts`:

```ts
export const readProjectHistory = (path: string) => invoke<string>("read_project_history", { path });
export const writeProjectHistory = (path: string, json: string) =>
  invoke<void>("write_project_history", { path, json });
```

- [ ] **Step 3: Integrate appStore.setActiveModule with history**

In `appStore.ts`, wrap:

```ts
setActiveModule: (next) => {
  const prev = get().activeModule;
  if (prev === next) return;
  useHistoryStore.getState().push({
    label: `Switch to ${next}`,
    at: Date.now(),
    do: () => set({ activeModule: next }),
    undo: () => set({ activeModule: prev }),
  });
},
```

- [ ] **Step 4: Write integration test**

`src/stores/storeIntegration.test.ts`:

```ts
import { beforeEach, describe, expect, it } from "vitest";
import { useAppStore } from "@/stores/appStore";
import { useHistoryStore } from "@/stores/historyStore";

describe("appStore / historyStore integration", () => {
  beforeEach(() => {
    useAppStore.setState({ activeModule: "website" });
    useHistoryStore.getState().clear();
  });

  it("pushes a command when module changes", () => {
    useAppStore.getState().setActiveModule("video");
    expect(useAppStore.getState().activeModule).toBe("video");
    expect(useHistoryStore.getState().past).toHaveLength(1);
  });

  it("undo reverts the module switch", () => {
    useAppStore.getState().setActiveModule("video");
    useHistoryStore.getState().undo();
    expect(useAppStore.getState().activeModule).toBe("website");
  });

  it("redo re-applies", () => {
    useAppStore.getState().setActiveModule("video");
    useHistoryStore.getState().undo();
    useHistoryStore.getState().redo();
    expect(useAppStore.getState().activeModule).toBe("video");
  });

  it("serialize + hydrate round-trips labels", () => {
    useAppStore.getState().setActiveModule("typography");
    const raw = useHistoryStore.getState().serialize();
    useHistoryStore.getState().clear();
    useHistoryStore.getState().hydrate(raw);
    const past = useHistoryStore.getState().past;
    expect(past).toHaveLength(1);
    expect(past[0].label).toContain("typography");
  });
});
```

- [ ] **Step 5: Run tests**

```bash
pnpm test -- --run src/stores/storeIntegration.test.ts 2>&1 | tail -10
```
Expected: all 4 PASS.

- [ ] **Step 6: Final verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -3 && cargo test --lib 2>&1 | tail -3
cd .. && pnpm exec tsc --noEmit && pnpm biome check . 2>&1 | tail -3
```

```bash
git add src/stores/historyStore.ts src/stores/appStore.ts src/stores/projectStore.ts \
        src/stores/storeIntegration.test.ts src/lib/projectCommands.ts \
        src-tauri/src/projects/commands.rs src-tauri/src/lib.rs
git commit -m "feat(core-ui): Undo/Redo-Persistenz + Store-Integration

- historyStore.serialize/hydrate schreiben/lesen history.json via neue
  read_project_history/write_project_history Tauri-Commands
- projectStore.openProject hydriert History statt sie zu löschen
- appStore.setActiveModule pusht undoable Commands
- 4 Integration-Tests

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 8: Claude Vision implementation

**Closes:** P2 #79.

**Files:**
- Modify: `src-tauri/src/api_clients/claude.rs` — extend `send_messages` to accept image content blocks via `request.payload.images[]`
- Modify: `src-tauri/src/taste_engine/analyzer.rs` — implement `ClaudeVisionAnalyzer::analyze` against a real `ClaudeClient`

**Approach:** Anthropic Messages API Vision format:
```json
{
  "messages": [{
    "role": "user",
    "content": [
      { "type": "text", "text": "…" },
      { "type": "image", "source": { "type": "base64", "media_type": "image/jpeg", "data": "…" } }
    ]
  }]
}
```

`AiRequest.payload.images` will be `[{ media_type: String, data: String /* base64 */ }]`. When present, Claude's `send_messages` builds a multi-block `content` array; otherwise the existing single-string path stays.

- [ ] **Step 1: Write wiremock test for Vision payload shape**

Extend `src-tauri/src/api_clients/claude.rs` tests:

```rust
#[tokio::test]
async fn vision_payload_uses_image_block() {
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(body_partial_json(json!({
            "messages": [
                {
                    "content": [
                        { "type": "text", "text": "Describe palette" },
                        { "type": "image", "source": { "type": "base64", "media_type": "image/png", "data": "AAAA" } }
                    ]
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "m1",
            "content": [{ "type": "text", "text": "Warm reds, cream." }],
            "stop_reason": "end_turn",
            "usage": { "input_tokens": 10, "output_tokens": 5 }
        })))
        .mount(&server)
        .await;

    let ks: Arc<dyn KeyStore> = Arc::new(InMemoryKeyStore::seeded("claude", "k"));
    let client = ClaudeClient::for_test(ks, server.uri());
    let resp = client
        .execute(AiRequest {
            prompt: "Describe palette".into(),
            task: TaskKind::ImageAnalysis, // add this TaskKind if not present
            complexity: Complexity::Simple,
            payload: json!({
                "images": [{ "media_type": "image/png", "data": "AAAA" }]
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(resp.payload.to_string().contains("Warm reds"));
}
```

- [ ] **Step 2: If `ImageAnalysis` TaskKind doesn't exist, add it**

Extend `src-tauri/src/ai_router/models.rs:TaskKind`:

```rust
    /// Image analysis via a vision-capable text model (Claude Vision).
    ImageAnalysis,
```

Add routing in `src-tauri/src/ai_router/router.rs`:

```rust
(ImageAnalysis, _) => RouteDecision::with_fallbacks(ClaudeSonnet, vec![ClaudeHaiku]),
```

- [ ] **Step 3: Implement Vision path in claude.rs**

Update `send_messages` — if `request.payload.get("images")` is a non-empty array, build multi-block content; otherwise keep the existing string path:

```rust
let content = if let Some(imgs) = request.payload.get("images").and_then(|v| v.as_array()) {
    let mut blocks = vec![json!({ "type": "text", "text": request.prompt.clone() })];
    for img in imgs {
        let media_type = img.get("media_type").and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Permanent("claude vision: media_type required".into()))?;
        let data = img.get("data").and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Permanent("claude vision: data required".into()))?;
        blocks.push(json!({
            "type": "image",
            "source": { "type": "base64", "media_type": media_type, "data": data }
        }));
    }
    serde_json::Value::Array(blocks)
} else {
    serde_json::Value::String(request.prompt.clone())
};

let body = json!({
    "model": slug,
    "max_tokens": 1024,
    "messages": [{ "role": "user", "content": content }]
});
```

- [ ] **Step 4: Implement ClaudeVisionAnalyzer**

`src-tauri/src/taste_engine/analyzer.rs`:

Replace the "scheduled for Phase 3+" stub with a real implementation that:
1. Reads the image file from disk
2. Base64-encodes
3. Sends an `AiRequest{ task: ImageAnalysis, prompt: "Extract palette + style keywords as JSON {palette: [\"#rrggbb\"], style: [\"…\"]}", payload: images }`
4. Parses the text response as JSON (fallback to regex if Claude replies in prose)

```rust
#[async_trait]
impl VisionAnalyzer for ClaudeVisionAnalyzer {
    async fn analyze(&self, path: &Path) -> Result<ImageAnalysis, TasteError> {
        let bytes = std::fs::read(path).map_err(|e| TasteError::Analysis(e.to_string()))?;
        let media_type = mime_for(path);
        let data = base64::encode(&bytes);

        let req = AiRequest {
            prompt: r#"Extract the dominant palette (up to 6 hex colors) and 3–6 style keywords from this image. Respond strictly as JSON: {"palette": ["#rrggbb"], "style": ["…"]}."#.into(),
            task: TaskKind::ImageAnalysis,
            complexity: Complexity::Simple,
            payload: json!({ "images": [{ "media_type": media_type, "data": data }] }),
            ..Default::default()
        };
        let resp = self.router.route_request(req).await
            .map_err(|e| TasteError::Analysis(e.to_string()))?;
        let text = resp.payload.get("content")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|b| b.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");
        parse_palette_style_json(text)
    }
}

fn mime_for(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "image/png",
    }
}
```

Add a `ClaudeVisionAnalyzer::new(router: Arc<AiRouter>) -> Self`. Parse helper `parse_palette_style_json` uses serde with a fallback regex for hex codes.

Add `base64 = "0.22"` to `src-tauri/Cargo.toml` if absent.

- [ ] **Step 5: Run all tests**

```bash
cd src-tauri && cargo test 2>&1 | tail -15
```
Expected: all green including new `vision_payload_uses_image_block` + analyzer tests.

Add a small test for `ClaudeVisionAnalyzer::analyze` using wiremock to simulate a Claude response with `{"palette":["#abc"], "style":["warm"]}` in the text block.

- [ ] **Step 6: Final verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -3
cd .. && pnpm biome check . 2>&1 | tail -3
```

```bash
git add src-tauri/src/api_clients/claude.rs src-tauri/src/ai_router/models.rs \
        src-tauri/src/ai_router/router.rs src-tauri/src/taste_engine/analyzer.rs \
        src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat(taste-engine): Claude Vision für Bild-Analyse

- claude.rs akzeptiert payload.images[] und baut Multi-Block-Content
- Neue TaskKind::ImageAnalysis + Routing auf ClaudeSonnet/Haiku
- ClaudeVisionAnalyzer ersetzt 'scheduled for Phase 3+'-Stub

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 9: TasteWatcher live loop + Vision wiring

**Closes:** P2 #81. **Depends on:** Task 8.

**Files:**
- Modify: `src-tauri/src/lib.rs:50-64` — spawn a background task that drains the watcher and calls `engine.refresh()`
- Modify: `src-tauri/src/taste_engine/mod.rs` — ensure `refresh()` chains parser + analyzer

- [ ] **Step 1: Switch analyzer to ClaudeVisionAnalyzer**

In `lib.rs` setup, replace:

```rust
Arc::new(StubVisionAnalyzer::new())
```
with
```rust
Arc::new(taste_engine::ClaudeVisionAnalyzer::new(ai_router.clone()))
```

(Make sure `ClaudeVisionAnalyzer` is re-exported from `taste_engine::mod.rs`.)

- [ ] **Step 2: Spawn watcher loop**

Still in `lib.rs` setup, after `app.manage(TasteEngineState::new(engine.clone()))`:

```rust
let watch_engine = engine.clone();
let watch_root = meingeschmack_root.clone();
tauri::async_runtime::spawn(async move {
    match taste_engine::watcher::TasteWatcher::try_new(&watch_root) {
        Ok(mut w) => {
            loop {
                if w.next_event().await.is_none() { break; }
                if let Err(e) = watch_engine.refresh().await {
                    eprintln!("[taste-engine] refresh failed: {e}");
                }
            }
        }
        Err(e) => eprintln!("[taste-engine] watcher init failed: {e}"),
    }
});
```

(Adjust method names once grepped. The exact `TasteWatcher` API was tested; read `src-tauri/src/taste_engine/watcher.rs` to confirm `try_new`/`next_event`/`try_drain` names.)

- [ ] **Step 3: Test the loop with a TempDir fixture**

Create `src-tauri/tests/taste_engine_live.rs`:

```rust
use std::sync::Arc;
use std::time::Duration;

use tempfile::TempDir;
use terryblemachine_lib::ai_router::{AiRouter, DefaultRoutingStrategy, PriorityQueue, RetryPolicy};
use terryblemachine_lib::taste_engine::{StubVisionAnalyzer, TasteEngine};
use tokio::time::sleep;

#[tokio::test]
async fn refresh_picks_up_new_rules_file() {
    let tmp = TempDir::new().unwrap();
    let engine = Arc::new(TasteEngine::new(
        tmp.path().to_path_buf(),
        Arc::new(StubVisionAnalyzer::new()),
    ));

    // Initially empty
    let _ = engine.refresh().await;
    let profile_before = engine.profile();

    std::fs::write(tmp.path().join("rules.md"), "- Prefer warm palettes\n").unwrap();
    sleep(Duration::from_millis(50)).await;
    engine.refresh().await.expect("refresh");
    let profile_after = engine.profile();

    assert!(
        profile_after.rules.preferred.len() > profile_before.rules.preferred.len(),
        "new rule should appear after refresh"
    );
}
```

- [ ] **Step 4: Run**

```bash
cd src-tauri && cargo test --test taste_engine_live 2>&1 | tail -10
```
Expected: PASS.

- [ ] **Step 5: Final verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -3
cd .. && pnpm biome check . 2>&1 | tail -3
```

```bash
git add src-tauri/src/lib.rs src-tauri/src/taste_engine/mod.rs src-tauri/tests/taste_engine_live.rs
git commit -m "feat(taste-engine): Live-Loop für meingeschmack/-Watcher

- lib.rs spawnt Tokio-Task der TasteWatcher::next_event konsumiert
- ClaudeVisionAnalyzer wird runtime-seitig registriert (statt Stub)
- Integration-Test verifiziert refresh() bei neuer Regel-Datei

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 10: Kling I2V, Runway Motion Brush, Ideogram v3

**Closes:** P2 #80.

**Files:**
- Modify: `src-tauri/src/api_clients/kling.rs` — add image-to-video endpoint
- Modify: `src-tauri/src/api_clients/runway.rs` — forward motion-brush mask/strokes payload
- Modify: `src-tauri/src/api_clients/ideogram.rs` — use v3 `model_version` parameter

For each client: add a wiremock test that asserts the new payload shape, then implement.

- [ ] **Step 1: Kling image-to-video — write failing test**

In `kling.rs` tests:

```rust
#[tokio::test]
async fn image_to_video_hits_image2video_endpoint() {
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/videos/image2video"))
        .and(body_partial_json(json!({"image_url":"https://src/a.png","prompt":"cinematic"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "task_id": "t1",
            "status": "succeeded",
            "video_url": "https://out/v.mp4"
        })))
        .mount(&server).await;
    let ks = Arc::new(InMemoryKeyStore::seeded("kling", "k"));
    let c = KlingClient::for_test(ks, server.uri());
    let resp = c.execute(AiRequest{
        prompt: "cinematic".into(),
        task: TaskKind::ImageToVideo,
        payload: json!({"image_url":"https://src/a.png"}),
        ..Default::default()
    }).await.unwrap();
    assert_eq!(resp.payload.get("video_url").and_then(|v| v.as_str()), Some("https://out/v.mp4"));
}
```

- [ ] **Step 2: Implement the endpoint dispatch**

In `KlingClient::execute`, branch on `request.task`:

```rust
let endpoint = match request.task {
    TaskKind::TextToVideo => "/v1/videos/text2video",
    TaskKind::ImageToVideo => "/v1/videos/image2video",
    _ => return Err(ProviderError::Permanent("kling: unsupported task".into())),
};
let body = match request.task {
    TaskKind::ImageToVideo => {
        let img = request.payload.get("image_url").and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Permanent("kling image2video: image_url required".into()))?;
        json!({ "image_url": img, "prompt": request.prompt })
    }
    _ => json!({ "prompt": request.prompt }),
};
```

- [ ] **Step 3: Runway motion brush — write failing test**

```rust
#[tokio::test]
async fn motion_brush_strokes_forwarded() {
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/image_to_video"))
        .and(body_partial_json(json!({
            "motion_brush": { "strokes": [{"x":10,"y":20,"dx":5,"dy":0}] }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "r1", "status": "succeeded", "output": ["https://out/v.mp4"]
        })))
        .mount(&server).await;
    // call client with payload containing motion_brush.strokes
}
```

- [ ] **Step 4: Forward `motion_brush` field in runway.rs**

If `request.payload.motion_brush` is present, include it in the outgoing body. Keep existing `image_url`/`prompt` fields.

- [ ] **Step 5: Ideogram v3 — write failing test**

```rust
#[tokio::test]
async fn v3_model_version_sent() {
    use wiremock::matchers::{body_partial_json, method, path};
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/generate"))
        .and(body_partial_json(json!({ "model_version": "V_3" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{ "url": "https://out/l.png" }]
        })))
        .mount(&server).await;
    // call client → expect success
}
```

- [ ] **Step 6: Add `model_version: "V_3"` to the ideogram body**

Look up Ideogram's actual request contract first — if v3 uses a different endpoint (`/v3/generate`), change URL; if it uses a header, add header; if it's a body field, add body field.

- [ ] **Step 7: Run client tests**

```bash
cd src-tauri && cargo test --lib api_clients 2>&1 | tail -15
```
Expected: all client tests green incl. 3 new ones.

- [ ] **Step 8: Final verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -3
cd .. && pnpm biome check . 2>&1 | tail -3
```

```bash
git add src-tauri/src/api_clients/kling.rs src-tauri/src/api_clients/runway.rs src-tauri/src/api_clients/ideogram.rs
git commit -m "feat(ai-router): Kling I2V, Runway Motion Brush, Ideogram v3

- Kling dispatch auf /v1/videos/image2video bei ImageToVideo
- Runway forwardet payload.motion_brush (strokes) im Body
- Ideogram Request trägt model_version=V_3

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 11: URL-Analyzer asset download + Builder UI wiring

**Closes:** P3 #82.

**Files:**
- Modify: `scripts/url_analyzer.mjs` — download images/icons/fonts into `<project>/assets/`
- Modify: `src-tauri/src/website_analyzer/playwright.rs` — pass optional `assets_dir` to the sidecar
- Modify: `src/pages/WebsiteBuilder.tsx` — add URL input + "Analyze" button
- Test: `src-tauri/src/website_analyzer/playwright.rs` — extend existing test

- [ ] **Step 1: Extend sidecar to download assets**

In `scripts/url_analyzer.mjs`, after the screenshot step:

```js
if (assetsDir) {
  await fs.mkdir(assetsDir, { recursive: true });
  const urls = await page.evaluate(() => {
    const out = new Set();
    document.querySelectorAll("img[src]").forEach((img) => out.add(img.src));
    document.querySelectorAll("link[rel='icon'][href]").forEach((l) => out.add(l.href));
    performance.getEntriesByType("resource")
      .filter((r) => r.initiatorType === "css" && /\.(woff2?|ttf|otf)$/i.test(r.name))
      .forEach((r) => out.add(r.name));
    return Array.from(out);
  });
  for (const u of urls.slice(0, 50)) {
    try {
      const r = await fetch(u);
      if (!r.ok) continue;
      const buf = Buffer.from(await r.arrayBuffer());
      const safe = u.replace(/[^a-z0-9._-]/gi, "_").slice(-120);
      await fs.writeFile(path.join(assetsDir, safe), buf);
    } catch {}
  }
}
```

Read the sidecar header to add `--assets-dir` CLI arg parsing.

- [ ] **Step 2: Pass assets_dir from Rust**

In `playwright.rs`, extend `PlaywrightUrlAnalyzer::analyze(url)` signature — if not yet accepting a project directory, add `project_root: Option<PathBuf>` and when set spawn sidecar with `--assets-dir <project_root>/assets`.

Update `analyze_url` Tauri command to accept optional `project_path`.

- [ ] **Step 3: Add URL input to WebsiteBuilder**

In `src/pages/WebsiteBuilder.tsx`, add above the prompt:

```tsx
<div className="flex items-end gap-2">
  <div className="flex-1">
    <Input
      label="Reference URL (optional)"
      id="website-ref-url"
      placeholder="https://stripe.com"
      value={refUrl}
      onValueChange={setRefUrl}
    />
  </div>
  <Button
    variant="secondary"
    onClick={async () => {
      try {
        setAnalyzing(true);
        const result = await analyzeUrl(refUrl);
        setAnalysis(result);
        notify({ kind: "success", message: "URL analyzed" });
      } catch (e) {
        notify({ kind: "error", message: "URL analysis failed", detail: String(e) });
      } finally { setAnalyzing(false); }
    }}
    disabled={!refUrl || analyzing}
  >
    {analyzing ? "Analyzing…" : "Analyze URL"}
  </Button>
</div>
```

Pass `analysis` into the generator call (wire through `generateWebsite({ prompt, reference: analysis, module })`).

- [ ] **Step 4: Extend Rust test for asset download**

In `src-tauri/src/website_analyzer/playwright.rs` tests, or in a new integration test, stand up a `wiremock::MockServer` serving a small HTML with `<img>` + `<link rel=icon>`, and verify files appear in `assets_dir` after analyze.

- [ ] **Step 5: Run tests**

```bash
cd src-tauri && cargo test website_analyzer 2>&1 | tail -10
cd .. && pnpm test -- --run 2>&1 | tail -5
```

- [ ] **Step 6: Final verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -3
cd .. && pnpm exec tsc --noEmit && pnpm biome check . 2>&1 | tail -3
```

```bash
git add scripts/url_analyzer.mjs src-tauri/src/website_analyzer/playwright.rs \
        src-tauri/src/website_analyzer/commands.rs src/pages/WebsiteBuilder.tsx \
        src/lib/websiteCommands.ts
git commit -m "feat(website): URL-Analyzer Asset-Download + Builder-UI-Input

- Sidecar lädt Bilder/Icons/Fonts (max 50) nach <project>/assets/
- Rust reicht project_path an den Sidecar als --assets-dir weiter
- WebsiteBuilder hat URL-Input + Analyze-Button; Analyse wird an den
  Generator als reference: AnalysisResult übergeben

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 12: Claude-Assist inline-edit

**Closes:** P3 #83.

**Files:**
- Create: `src-tauri/src/code_generator/assist.rs` — `modify_selection(file_path, selection, instruction, files)` command
- Modify: `src-tauri/src/lib.rs` — register command
- Modify: `src/components/website/CodeEditor.tsx` — add selection-based "Modify…" popover
- Create: `src/components/website/AssistPopover.tsx`

- [ ] **Step 1: Rust command**

`assist.rs`:

```rust
//! Inline-edit a code selection via Claude.
//!
//! Input: the current `files` array, the target file path, the selected
//! text range (start/end line+col), and the user's natural-language
//! instruction. Claude returns the replacement for the selected range only.

use serde::{Deserialize, Serialize};

use crate::ai_router::{AiRequest, AiRouter, Complexity, TaskKind};
use crate::code_generator::types::GeneratedFile;

#[derive(Debug, Deserialize)]
pub struct ModifyRequest {
    pub files: Vec<GeneratedFile>,
    pub file_path: String,
    pub selection: String,
    pub instruction: String,
}

#[derive(Debug, Serialize)]
pub struct ModifyResponse {
    pub replacement: String,
}

pub async fn modify_selection(
    router: &AiRouter,
    req: ModifyRequest,
) -> Result<ModifyResponse, String> {
    let prompt = format!(
        "You are editing `{file}` in a React+Tailwind project. Here is the selected snippet:\n\n```\n{sel}\n```\n\nApply this change: {instr}\n\nReturn ONLY the replacement snippet, no prose, no code fences.",
        file = req.file_path,
        sel = req.selection,
        instr = req.instruction,
    );
    let ai_req = AiRequest {
        prompt,
        task: TaskKind::TextGeneration,
        complexity: Complexity::Medium,
        ..Default::default()
    };
    let resp = router.route_request(ai_req).await.map_err(|e| e.to_string())?;
    let text = resp.payload.get("content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|b| b.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    Ok(ModifyResponse { replacement: text })
}

#[tauri::command]
pub async fn modify_code_selection(
    state: tauri::State<'_, crate::ai_router::commands::AiRouterState>,
    req: ModifyRequest,
) -> Result<ModifyResponse, String> {
    modify_selection(&state.0, req).await
}
```

Register in `lib.rs`: `code_generator::assist::modify_code_selection`.

Add unit tests wiremocking an AiClient that returns a known replacement.

- [ ] **Step 2: Frontend wrapper**

`src/lib/websiteCommands.ts`:

```ts
export interface ModifyRequestInput {
  files: { path: string; content: string }[];
  filePath: string;
  selection: string;
  instruction: string;
}
export interface ModifyResponseOutput { replacement: string }
export const modifyCodeSelection = (req: ModifyRequestInput) =>
  invoke<ModifyResponseOutput>("modify_code_selection", { req });
```

- [ ] **Step 3: AssistPopover**

```tsx
// src/components/website/AssistPopover.tsx
import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";

interface Props {
  selection: string;
  onSubmit: (instruction: string) => Promise<void>;
  onClose: () => void;
  busy: boolean;
}

export function AssistPopover({ selection, onSubmit, onClose, busy }: Props) {
  const [instruction, setInstruction] = useState("");
  return (
    <div className="fixed inset-0 z-30 flex items-center justify-center bg-neutral-dark-950/60">
      <div className="w-[28rem] rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 p-4">
        <div className="mb-2 font-mono text-2xs uppercase tracking-label text-accent-500">
          Modify selection
        </div>
        <pre className="mb-3 max-h-32 overflow-auto rounded-xs bg-neutral-dark-950 p-2 text-2xs text-neutral-dark-300">
          {selection.slice(0, 500)}
        </pre>
        <Input
          label="Change to"
          id="assist-instruction"
          placeholder="Make headline larger and center it"
          value={instruction}
          onValueChange={setInstruction}
        />
        <div className="mt-3 flex justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={onClose} disabled={busy}>
            Cancel
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={() => onSubmit(instruction)}
            disabled={!instruction || busy}
          >
            {busy ? "Thinking…" : "Apply"}
          </Button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Wire into CodeEditor**

Read `src/components/website/CodeEditor.tsx`. Add: toolbar button "Modify selection" (Cmd+K) — reads Monaco's `editor.getModel().getValueInRange(editor.getSelection())`, opens AssistPopover, on submit calls `modifyCodeSelection`, then replaces the range via `editor.executeEdits`.

- [ ] **Step 5: Tests**

Rust: wiremocked AiClient, assert `modify_selection` returns the client's text response.

TS: component test for `AssistPopover` — renders selection, disables Apply when empty, calls onSubmit with instruction.

- [ ] **Step 6: Final verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -3 && cargo test --lib 2>&1 | tail -3
cd .. && pnpm exec tsc --noEmit && pnpm biome check . 2>&1 | tail -3
```

```bash
git add src-tauri/src/code_generator/assist.rs src-tauri/src/code_generator/mod.rs \
        src-tauri/src/lib.rs src/lib/websiteCommands.ts \
        src/components/website/AssistPopover.tsx src/components/website/CodeEditor.tsx
git commit -m "feat(website): Claude-Assist Inline-Edit

- Rust modify_code_selection Command: Claude ersetzt nur den
  markierten Bereich via TaskKind::TextGeneration
- AssistPopover im Code-Editor: Cmd+K → Instruction → Ersetzen

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 13: Website Export Dialog + richer scaffolds

**Closes:** P3 #84.

**Files:**
- Create: `src/components/website/WebsiteExportDialog.tsx`
- Modify: `src/pages/WebsiteBuilder.tsx` — add Export button + dialog
- Modify: `src-tauri/src/exporter/zip_export.rs` — richer React + Next.js scaffolds + optional Vercel/Netlify configs

- [ ] **Step 1: Expand React scaffold**

In `zip_export.rs:react_scaffold()`, add:
- `index.html` entry (with `<div id="root">` + `<script type="module" src="/src/main.jsx">`)
- `src/main.jsx` mounting `App`
- `src/App.jsx` that imports the generator's produced `index.html`-as-component-shell (realistically: a simple `App` that renders a static note "Generated site — open index.html directly in dev"; if the generator produces React components, mount those; keep it minimal but buildable)
- `tailwind.config.js`, `postcss.config.js`, `src/index.css` with `@tailwind` directives

- [ ] **Step 2: Expand Next.js scaffold**

Add:
- `app/layout.tsx` with basic HTML shell
- `app/page.tsx` that loads generated content
- `tailwind.config.ts` + `app/globals.css`

- [ ] **Step 3: Optional deploy configs**

Add two enum variants `ExportFormat::Vercel` / `ExportFormat::Netlify` (or a `deploy_config: Option<DeployTarget>` field on `ExportRequest`) that write `vercel.json` / `netlify.toml`.

`vercel.json`:
```json
{ "framework": "vite", "buildCommand": "pnpm build", "outputDirectory": "dist" }
```

`netlify.toml`:
```toml
[build]
  command = "pnpm build"
  publish = "dist"
```

Update existing tests + add one per new format.

- [ ] **Step 4: Export Dialog**

```tsx
// src/components/website/WebsiteExportDialog.tsx
import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Dropdown } from "@/components/ui/Dropdown";
import { Modal } from "@/components/ui/Modal";

export type WebsiteExportFormat = "raw" | "react" | "next-js";

interface Props {
  open: boolean;
  onClose: () => void;
  onExport: (settings: { format: WebsiteExportFormat; deploy?: "vercel" | "netlify" }) => void;
}

export function WebsiteExportDialog({ open, onClose, onExport }: Props) {
  const [format, setFormat] = useState<WebsiteExportFormat>("raw");
  const [deploy, setDeploy] = useState<"none" | "vercel" | "netlify">("none");
  return (
    <Modal open={open} onClose={onClose} title="Export website">
      <Dropdown
        value={format}
        onChange={(v) => setFormat(v as WebsiteExportFormat)}
        options={[
          { value: "raw", label: "Standalone HTML/CSS/JS" },
          { value: "react", label: "React (Vite)" },
          { value: "next-js", label: "Next.js (App Router)" },
        ]}
      />
      <Dropdown
        value={deploy}
        onChange={(v) => setDeploy(v as typeof deploy)}
        options={[
          { value: "none", label: "No deploy config" },
          { value: "vercel", label: "+ vercel.json" },
          { value: "netlify", label: "+ netlify.toml" },
        ]}
      />
      <div className="mt-4 flex justify-end gap-2">
        <Button variant="ghost" onClick={onClose}>Cancel</Button>
        <Button
          variant="primary"
          onClick={() =>
            onExport({
              format,
              deploy: deploy === "none" ? undefined : deploy,
            })
          }
        >
          Export
        </Button>
      </div>
    </Modal>
  );
}
```

- [ ] **Step 5: Wire into WebsiteBuilder**

Add an Export button; on submit call `exportWebsite({ project, format, destination, deploy })`.

Destination picker: if Tauri `dialog::save` plugin available, use it; otherwise prompt user path via Input. Minimum: default to `~/Downloads`.

- [ ] **Step 6: Tests**

Rust:
- `vercel_config_present_when_requested` / `netlify_config_present_when_requested`
- `react_scaffold_contains_main_jsx_and_tailwind_config`
- `nextjs_scaffold_contains_layout_and_page`

Frontend: WebsiteExportDialog test — user picks format + deploy, onExport called with expected settings.

- [ ] **Step 7: Final verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -3 && cargo test --lib 2>&1 | tail -3
cd .. && pnpm exec tsc --noEmit && pnpm biome check . 2>&1 | tail -3
```

```bash
git add src-tauri/src/exporter/zip_export.rs src-tauri/src/exporter/mod.rs \
        src/components/website/WebsiteExportDialog.tsx \
        src/components/website/WebsiteExportDialog.test.tsx \
        src/pages/WebsiteBuilder.tsx src/lib/websiteCommands.ts
git commit -m "feat(export): Website-Export-Dialog + Vite/Next.js-Scaffolds + Deploy-Configs

- React-Bundle: index.html + src/main.jsx + App.jsx + Tailwind-Setup
- Next.js-Bundle: app/layout.tsx + app/page.tsx + globals.css
- Optional vercel.json / netlify.toml
- UI: WebsiteExportDialog mit Format+Deploy-Dropdown

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 14: Desktop preview 1920px + Editor/Builder tests

**Closes:** P3 #85.

**Files:**
- Modify: `src/components/website/DevicePreview.tsx` — `desktop: 1920`
- Create: `src/components/website/CodeEditor.test.tsx`
- Create: `src/pages/WebsiteBuilder.test.tsx`

- [ ] **Step 1: Change desktop width**

`DevicePreview.tsx`:
```ts
const DEVICE_WIDTHS = { desktop: 1920, tablet: 768, mobile: 375 } as const;
```

- [ ] **Step 2: CodeEditor test**

Since Monaco is mocked in tests, assert: renders a textarea stub, calls `onChange` when text typed, respects `language` prop (check passed-through value).

- [ ] **Step 3: WebsiteBuilder test**

Assert: prompt input visible; URL input visible; generate button disabled when prompt empty; file list empty by default; after mock `generateWebsite` returns, file tabs appear and first file's content shows in editor stub.

Mock `generateWebsite`, `analyzeUrl`, `exportWebsite` via `vi.mock("@/lib/websiteCommands", …)`.

- [ ] **Step 4: Run**

```bash
pnpm test -- --run src/components/website src/pages/WebsiteBuilder.test.tsx 2>&1 | tail -10
```

- [ ] **Step 5: Final verify + commit**

```bash
pnpm exec tsc --noEmit && pnpm biome check . 2>&1 | tail -3
```

```bash
git add src/components/website/DevicePreview.tsx src/components/website/CodeEditor.test.tsx \
        src/pages/WebsiteBuilder.test.tsx
git commit -m "fix(website): Desktop-Preview 1920px + Tests für Editor/Builder

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 15: Router fallbacks + Flux-2-Pro rename

**Closes:** P4 #92.

**Files:**
- Modify: `src-tauri/src/ai_router/models.rs` — rename `FalFluxPro` → `FalFlux2Pro` (or add new variant; keep old as deprecated alias)
- Modify: `src-tauri/src/api_clients/fal.rs` — endpoint `/fal-ai/flux/v2-pro` (verify current fal.ai catalog — if v2-pro unavailable, revert to `FalFluxPro` and update the plan/doc instead, noting the decision)
- Modify: `src-tauri/src/ai_router/router.rs` — add Replicate fallback to Simple/Complex + ImageEdit routes

- [ ] **Step 1: Decide naming**

Grep fal.ai current endpoints: `grep -rn "fal-ai/flux" src-tauri/src/api_clients/fal.rs`. If `fal-ai/flux-pro` is what the service calls "Flux 1.1 Pro" (now standard), **don't fabricate a v2 endpoint** — instead update the plan doc comment. Write a note in `docs/ENTWICKLUNG-SCHRITT-FUER-SCHRITT.md` (or as a code comment) that "Flux 2 Pro" in the plan refers to the current top-tier Flux endpoint, currently `flux-pro`.

If fal.ai genuinely has a v2 endpoint at time of implementation, add it as `FalFlux2Pro` variant; keep `FalFluxPro` for back-compat. Add routing strategy so `ImageGeneration, Complex` uses `FalFlux2Pro` with fallback to `FalFluxPro` then `ReplicateFluxDev`.

- [ ] **Step 2: Extend fallbacks in router**

```rust
(ImageGeneration, Simple) => {
    RouteDecision::with_fallbacks(FalSdxl, vec![FalFluxPro, ReplicateFluxDev])
}
(ImageGeneration, Complex) => {
    RouteDecision::with_fallbacks(FalFluxPro, vec![ReplicateFluxDev])
}
(ImageGeneration, Medium) => {
    RouteDecision::with_fallbacks(FalFluxPro, vec![ReplicateFluxDev])
}

(ImageEdit, _) => RouteDecision::with_fallbacks(FalFluxPro, vec![ReplicateFluxDev]),
```

- [ ] **Step 3: Update router tests**

Extend `router.rs` tests:

```rust
#[test]
fn image_edit_falls_back_to_replicate() {
    let r = DefaultRoutingStrategy;
    let d = r.select(&AiRequest { task: TaskKind::ImageEdit, complexity: Complexity::Medium, ..Default::default() });
    assert_eq!(d.primary, Model::FalFluxPro);
    assert!(d.fallbacks.contains(&Model::ReplicateFluxDev));
}
```

Add similar tests for Simple/Complex.

- [ ] **Step 4: Run**

```bash
cd src-tauri && cargo test --lib ai_router 2>&1 | tail -10
```

- [ ] **Step 5: Final verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -3
cd .. && pnpm biome check . 2>&1 | tail -3
```

```bash
git add src-tauri/src/ai_router/router.rs src-tauri/src/ai_router/models.rs
git commit -m "fix(ai-router): Replicate-Fallback für Simple/Complex/ImageEdit

- Vorher nur (ImageGeneration, Medium) mit Fallback
- Jetzt fallen Simple/Complex/ImageEdit auf ReplicateFluxDev zurück
- Flux-2-Pro-Naming siehe docs/ENTWICKLUNG-SCHRITT-FUER-SCHRITT.md-Kommentar

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 16: Inpainting end-to-end

**Closes:** P4 #88. **Depends on:** Tasks 1, 2.

**Files:**
- Modify: `src-tauri/src/image_pipeline/types.rs` — add `InpaintInput` + `ImagePipeline::inpaint`
- Modify: `src-tauri/src/image_pipeline/pipeline.rs` — implement via `TaskKind::Inpaint`
- Modify: `src-tauri/src/image_pipeline/stub.rs` — stub for the new method
- Modify: `src-tauri/src/image_pipeline/commands.rs` — `inpaint_image` Tauri command
- Modify: `src/lib/imageCommands.ts` — `inpaintImage` wrapper
- Modify: `src/components/graphic2d/FabricCanvas.tsx` — mask-drawing mode + `getMaskDataUrl()` method on handle
- Modify: `src/pages/Graphic2D.tsx` — "Inpaint" toolbar button + prompt dialog

- [ ] **Step 1: Add InpaintInput type**

```rust
// types.rs
#[derive(Debug, Clone, Deserialize)]
pub struct InpaintInput {
    pub prompt: String,
    pub source_url: String,
    pub mask_url: String,
    #[serde(default = "default_module")]
    pub module: String,
}

#[async_trait]
pub trait ImagePipeline: Send + Sync {
    // …existing…
    async fn inpaint(&self, input: InpaintInput) -> Result<ImageResult, ImagePipelineError>;
}
```

- [ ] **Step 2: Router implementation**

In `pipeline.rs`:

```rust
async fn inpaint(&self, input: InpaintInput) -> Result<ImageResult, ImagePipelineError> {
    let enriched = self.enrich(&input.prompt, &input.module);
    let req = AiRequest {
        prompt: enriched,
        task: TaskKind::Inpaint,
        complexity: Complexity::Medium,
        payload: json!({ "image_url": input.source_url, "mask_url": input.mask_url }),
        ..Default::default()
    };
    let resp = self.router.route_request(req).await
        .map_err(|e| ImagePipelineError::Router(e.to_string()))?;
    let (url, w, h) = first_image_url(&resp)
        .ok_or_else(|| ImagePipelineError::Provider("inpaint: no image".into()))?;
    Ok(ImageResult { url, width: w, height: h, seed: None, model: resp.model, cached: false })
}
```

- [ ] **Step 3: Stub + tests**

Stub returns `stub://image/inpaint/<hash>` URL; record calls.

Integration test in `src-tauri/tests/image_pipeline_integration.rs` — extend with an inpaint case using the StubFalClient.

- [ ] **Step 4: Tauri command**

```rust
#[tauri::command]
pub async fn inpaint_image(
    state: tauri::State<'_, ImagePipelineState>,
    input: InpaintInput,
) -> Result<ImageResult, IpcError> {
    state.0.inpaint(input).await.map_err(Into::into)
}
```

Register in `lib.rs`.

- [ ] **Step 5: Frontend mask drawing**

In `FabricCanvas.tsx`, add to the handle:

```ts
enterMaskMode: () => void;     // switches Fabric to free-drawing brush, white on transparent
exitMaskMode: () => void;
getMaskDataUrl: () => string;  // extract drawing layer as PNG data URL (white = inpaint, transparent = keep)
hasMask: () => boolean;
clearMask: () => void;
```

Implementation: overlay a `fabric.Canvas` layer or use `canvas.isDrawingMode = true` with `freeDrawingBrush.color = 'rgba(255,255,255,0.8)'` + `freeDrawingBrush.width = 40`. On `getMaskDataUrl`, render only the drawn paths by toggling non-path object visibility temporarily.

- [ ] **Step 6: Graphic2D UI**

New toolbar button "Inpaint". On click: enter mask mode, show a prompt input, on submit:
1. `canvasRef.current.getMaskDataUrl()` → data URL
2. Upload the source image data URL too (selected image as source_url)
3. Call `inpaintImage({ prompt, source_url, mask_url })`
4. On success: replace the source image with the inpaint result

(Note: fal.ai requires hosted URLs, not data URLs. Either base64-inline the image in the `image_url`/`mask_url` field if the endpoint supports it, or add a temporary upload step via a Tauri command that stores the data URL in `~/Library/Caches/terryblemachine/` and serves a `file://` URL. Simpler: accept that the real fal.ai call requires a hosted URL and surface a user-facing error when data URLs are detected — full upload pipeline is a separate future concern. Document the limitation in a code comment.)

- [ ] **Step 7: Tests**

Rust: `inpaint_routes_to_fal_flux_fill`, stub records the prompt.

Frontend: Fabric mask mode tests (given that jsdom can't actually draw, assert the `isDrawingMode` flag is set on enter and cleared on exit; test `hasMask()` returns false initially).

- [ ] **Step 8: Final verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -3 && cargo test 2>&1 | tail -3
cd .. && pnpm exec tsc --noEmit && pnpm biome check . 2>&1 | tail -3
```

```bash
git add src-tauri/src/image_pipeline src-tauri/tests/image_pipeline_integration.rs src-tauri/src/lib.rs \
        src/lib/imageCommands.ts src/components/graphic2d/FabricCanvas.tsx src/pages/Graphic2D.tsx
git commit -m "feat(graphic2d): Inpainting end-to-end

- ImagePipeline::inpaint → TaskKind::Inpaint → FalFluxFill
- Tauri inpaint_image Command + imageCommands.ts Wrapper
- FabricCanvas Mask-Modus (free-drawing) + getMaskDataUrl()
- Graphic2D Toolbar-Button + Prompt-Dialog
- Bekannte Grenze: fal.ai braucht gehostete URLs — data-URL
  Upload-Pfad ist dokumentiert aber deferred

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 17: Flip / Crop / Resize / Lasso / Rectangle selection

**Closes:** P4 #89.

**Files:**
- Modify: `src/components/graphic2d/FabricCanvas.tsx` — expose `flipH`, `flipV`, `setCanvasSize`, `cropToSelection`, `enterSelectionMode(kind)`, `clearSelection`
- Modify: `src/pages/Graphic2D.tsx` — toolbar buttons

- [ ] **Step 1: Handle extensions**

```ts
flipH: (id: string) => void;     // active object flipX
flipV: (id: string) => void;
setCanvasSize: (w: number, h: number) => void;
cropToSelection: () => void;      // clamp canvas to bounding box of active selection/marquee
enterMarqueeSelect: () => void;
enterLassoSelect: () => void;
exitSelectionMode: () => void;
```

Rectangle select: Fabric's default is already marquee-like (drag on empty canvas). Add "marquee mode" that yields a `fabric.Rect` used as a crop boundary.

Lasso: draw a `fabric.Path` via free-drawing; on drawing:ended convert to a polygon, use its bounding box for "crop to lasso" or pass points to selections.

- [ ] **Step 2: UI buttons**

In Graphic2D toolbar, add:
- Flip H (when object selected)
- Flip V
- Crop (disabled until a marquee/lasso exists)
- Canvas size inputs (width/height number fields)
- Selection mode toggle: Marquee / Lasso / Off

- [ ] **Step 3: Tests**

Component test: enterMarqueeSelect sets `canvas.isDrawingMode = false` and assigns a handler; calling handle.flipH(id) sets `obj.flipX = true`.

- [ ] **Step 4: Final verify + commit**

```bash
pnpm exec tsc --noEmit && pnpm biome check . 2>&1 | tail -3 && pnpm test -- --run 2>&1 | tail -3
```

```bash
git add src/components/graphic2d/FabricCanvas.tsx src/components/graphic2d/FabricCanvas.test.tsx src/pages/Graphic2D.tsx
git commit -m "feat(graphic2d): Flip/Crop/Resize + Marquee/Lasso-Selection

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 18: Text overlay Font / Color / Size picker

**Closes:** P4 #90.

**Files:**
- Create: `src/components/graphic2d/TextControls.tsx` — renders when a text object is selected; Font (Google Fonts dropdown), Color, Size inputs
- Modify: `src/components/graphic2d/FabricCanvas.tsx` — handle exposes `updateText(id, {font?, color?, size?})`
- Modify: `src/pages/Graphic2D.tsx` — mount TextControls in the right panel above the LayerList when selection is text

- [ ] **Step 1: Google Fonts list**

Static list of ~30 popular Google Fonts (Inter, Roboto, Open Sans, Poppins, Playfair Display, Space Mono, …). For each, dynamically inject `<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=…">` when selected — append to `<head>` once, keyed by name to avoid duplicates.

Store the font list in `src/lib/googleFonts.ts`:

```ts
export const GOOGLE_FONTS = [
  "Inter", "Roboto", "Open Sans", "Poppins", "Space Mono", "Playfair Display",
  "Merriweather", "Oswald", "Raleway", "Montserrat", "Lato", "Source Code Pro",
  "Nunito", "Work Sans", "Fira Sans", "Crimson Text", "Libre Baskerville",
  "Abril Fatface", "IBM Plex Sans", "IBM Plex Mono", "DM Sans", "DM Mono",
  "Bebas Neue", "Anton", "Archivo", "Rubik", "Quicksand", "Karla", "Inconsolata",
] as const;

export type GoogleFont = (typeof GOOGLE_FONTS)[number];

export function injectGoogleFont(name: GoogleFont) {
  const id = `gfont-${name.replace(/\s/g, "-")}`;
  if (document.getElementById(id)) return;
  const link = document.createElement("link");
  link.id = id;
  link.rel = "stylesheet";
  link.href = `https://fonts.googleapis.com/css2?family=${encodeURIComponent(name)}:wght@400;700&display=swap`;
  document.head.appendChild(link);
}
```

- [ ] **Step 2: Handle API**

```ts
updateText: (id: string, patch: { font?: string; color?: string; size?: number }) => void;
```

Implementation: find Textbox by id, set `fontFamily`/`fill`/`fontSize`, `c.requestRenderAll()`, `refreshLayers()`.

- [ ] **Step 3: TextControls component**

```tsx
import { GOOGLE_FONTS, injectGoogleFont } from "@/lib/googleFonts";
// …
<Dropdown
  value={font}
  onChange={(v) => { injectGoogleFont(v); setFont(v); onChange({ font: v }); }}
  options={GOOGLE_FONTS.map((f) => ({ value: f, label: f }))}
/>
<Input type="number" label="Size" value={size} onValueChange={…} />
<input type="color" value={color} onChange={…} />
```

- [ ] **Step 4: Tests**

TextControls test: changes font → calls onChange with { font }.

- [ ] **Step 5: Final verify + commit**

```bash
pnpm exec tsc --noEmit && pnpm biome check . 2>&1 | tail -3 && pnpm test -- --run 2>&1 | tail -3
```

```bash
git add src/components/graphic2d/TextControls.tsx src/components/graphic2d/TextControls.test.tsx \
        src/components/graphic2d/FabricCanvas.tsx src/pages/Graphic2D.tsx src/lib/googleFonts.ts
git commit -m "feat(graphic2d): Text-Overlay Font/Color/Size-Picker

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 19: PDF + GIF export

**Closes:** P4 #91.

**Files:**
- Install: `pnpm add jspdf gif.js`
- Modify: `src/components/graphic2d/FabricCanvas.tsx` — `toPdf()`, `toGif(frames?)`
- Modify: `src/components/graphic2d/ExportDialog.tsx` — `pdf`, `gif` format options
- Modify: `src/pages/Graphic2D.tsx` — handle new formats

- [ ] **Step 1: Install deps**

```bash
pnpm add jspdf gif.js
pnpm add -D @types/gif.js || true
```

- [ ] **Step 2: toPdf**

```ts
toPdf: () => string; // returns data:application/pdf;base64,…
```

```ts
import { jsPDF } from "jspdf";
// …
toPdf() {
  const c = canvasRef.current; if (!c) return "";
  const pngUrl = c.toDataURL({ format: "png", multiplier: 1 });
  const pdf = new jsPDF({ orientation: c.getWidth() >= c.getHeight() ? "landscape" : "portrait", unit: "px", format: [c.getWidth(), c.getHeight()] });
  pdf.addImage(pngUrl, "PNG", 0, 0, c.getWidth(), c.getHeight());
  return pdf.output("dataurlstring");
},
```

- [ ] **Step 3: toGif**

```ts
toGif: (options?: { frames?: number; delayMs?: number }) => Promise<string>;
```

Static variant (`frames <= 1`): emit a 1-frame GIF from the current canvas.

```ts
import GIF from "gif.js";
// …
toGif: async ({ frames = 1, delayMs = 100 } = {}) => new Promise<string>((resolve) => {
  const c = canvasRef.current; if (!c) { resolve(""); return; }
  const gif = new GIF({ workers: 2, quality: 10, width: c.getWidth(), height: c.getHeight() });
  for (let i = 0; i < Math.max(1, frames); i++) {
    gif.addFrame(c.lowerCanvasEl, { copy: true, delay: delayMs });
  }
  gif.on("finished", (blob: Blob) => {
    const r = new FileReader();
    r.onload = () => resolve(r.result as string);
    r.readAsDataURL(blob);
  });
  gif.render();
}),
```

gif.js needs a worker script — set `workerScript` to `/gif.worker.js` and copy the package's `gif.worker.js` into `public/` during setup.

- [ ] **Step 4: ExportDialog format options**

```ts
const FORMAT_OPTIONS = [
  { value: "png", label: "PNG" },
  { value: "jpeg", label: "JPEG" },
  { value: "webp", label: "WebP" },
  { value: "svg", label: "SVG" },
  { value: "pdf", label: "PDF" },
  { value: "gif", label: "GIF" },
];
```

- [ ] **Step 5: Graphic2D handleExport additions**

```ts
case "pdf":
  dataUrl = handle.toPdf();
  break;
case "gif":
  dataUrl = await handle.toGif({ frames: settings.frames ?? 1 });
  break;
```

Extend `ExportSettings` to allow optional `frames?: number` when format is `gif`.

- [ ] **Step 6: Tests**

```ts
// ExportDialog.test.tsx
it("shows PDF and GIF in format options", () => {
  render(<ExportDialog open onClose={() => {}} onExport={() => {}} />);
  expect(screen.getByText(/PDF/)).toBeInTheDocument();
  expect(screen.getByText(/GIF/)).toBeInTheDocument();
});
```

- [ ] **Step 7: Final verify + commit**

```bash
pnpm exec tsc --noEmit && pnpm biome check . 2>&1 | tail -3 && pnpm test -- --run 2>&1 | tail -3
```

```bash
git add package.json pnpm-lock.yaml src/components/graphic2d src/pages/Graphic2D.tsx public/gif.worker.js
git commit -m "feat(graphic2d): PDF- und GIF-Export

- jspdf für PDF (Canvas → PNG → PDF page)
- gif.js für GIF (static default, multi-frame optional)
- ExportDialog FORMAT_OPTIONS enthält jetzt pdf + gif

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

---

## Task 20: Cross-phase verification + honest Phase-4 re-commit

**Files:**
- Create: `docs/superpowers/specs/2026-04-17-phase-1-4-verification-report.md`

**Approach:** Run full verify pipeline. For each original gap task (#74-#92), re-read the current code against the plan bullet and mark ✓/~/✗ with file:line evidence. If any ✗ remains, halt and surface — don't claim done.

- [ ] **Step 1: Full verify**

```bash
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd .. && pnpm exec tsc --noEmit && pnpm biome check . && pnpm test -- --run
```
Expected: all green.

- [ ] **Step 2: Write verification report**

Document each of the 19 tasks: the closing commit SHA, the evidence path, the status. Explicit "✗ remaining" list if any.

- [ ] **Step 3: Commit report + push**

```bash
git add docs/superpowers/specs/2026-04-17-phase-1-4-verification-report.md
git commit -m "docs: Verification-Report Phase 1-4 Gap-Closure

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>" && git push origin main && sleep 5 && gh run watch --exit-status $(gh run list --limit 1 --json databaseId -q '.[0].databaseId')
```

- [ ] **Step 4: Close all 19 gap-tasks in the task tracker**

Mark #74–#92 as completed via TaskUpdate. Only after Step 1+2 pass.

---

## Scope Check (self-review)

**Spec coverage:** Each of the 19 audit gap-tasks maps to exactly one numbered Task above:
- P1: #74→T4, #75→T5, #76→T6, #77→T7
- P2: #78→T1, #79→T8, #80→T10, #81→T9
- P3: #82→T11, #83→T12, #84→T13, #85→T14
- P4: #86→T2, #87→T3, #88→T16, #89→T17, #90→T18, #91→T19, #92→T15

**Placeholder scan:** No "TBD"/"TODO". Every step contains concrete code or exact commands. "Defer to future task" is explicit only for the data-URL upload pipeline in Task 16 (documented trade-off, not missing work).

**Type consistency:** `ImagePipeline` methods, `AiRequest`/`AiResponse`, `GeneratedFile`, `ExportFormat`, `FabricCanvasHandle` are used consistently across tasks. `TaskKind::ImageAnalysis` is introduced in T8 and consumed in T8 only. `FalFluxPro` vs `FalFlux2Pro` is explicitly handled in T15 with a decision gate.

**Risk areas:**
- **Task 10 Ideogram v3**: exact field/URL for v3 is service-dependent; the plan includes a "look up actual contract" step before coding.
- **Task 16 Inpainting**: data-URL → hosted-URL upload pipeline is deferred with an explicit comment; real cloud inpainting may require further work.
- **Task 19 gif.js worker**: requires copying `gif.worker.js` into `public/`; if Vite's asset handling differs, adjust path.

---

**Plan complete and saved to** `docs/superpowers/plans/2026-04-17-phase-1-4-gap-closure.md`.

Two execution options:

**1. Subagent-Driven (recommended)** — One fresh subagent per task, review between tasks, parallel where independent.

**2. Inline Execution** — Execute sequentially in this session with checkpoints.

Given the scope (20 tasks, spanning both Rust and TypeScript, with real TDD at each step), **subagent-driven** is strongly recommended.
