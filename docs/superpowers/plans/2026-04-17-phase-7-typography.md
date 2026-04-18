# Phase 7 — Typografie & Logos Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `/typography` module: text brief → Ideogram-generated logo variants → rasterised picks vectorised via VTracer → SVG editor + text controls → exported brand kit ZIP (all sizes + color variants + style guide).

**Architecture:**
- **Backend (Rust/Tauri):** `logo_pipeline` module (Ideogram variant generation), `vectorizer` module (`vtracer` Rust crate), `brand_kit` module (resize + color variants + style guide HTML + ZIP packaging).
- **Frontend:** `/typography` page with logo gallery + favorites, SVG editor reusing Fabric.js from Phase 4, text kerning/tracking controls, export dialog.
- **Reuses:** `api_clients/ideogram.rs` (v3 already wired from FU #99), `image_pipeline` (variants), `googleFonts.ts` from Phase 4 T18, `jspdf` from Phase 4 T19, `zip` crate from Phase 3 exporter.

**Tech Stack:** vtracer (Rust), Fabric.js (existing), jspdf (existing), @remotion/three (NOT used — pure vector work), Tailwind Industrial theme.

**Scope decisions:**
- **VTracer**: use official `vtracer` crate from Visioncortex. MIT license, pure-Rust, no Python sidecar.
- **SVG editor**: Fabric.js's existing SVG import/export via `toSVG()` + `loadSVGFromString`. Full path-level Bezier editing is a stretch — ship minimal SVG edit (move, scale, rotate, color change via Fabric object controls); document polygon/path editing as deferred if too scope-heavy.
- **Font browser**: reuse `src/lib/googleFonts.ts` (29 fonts from Phase 4 T18). Local system fonts deferred — requires native font enumeration via Tauri plugin.
- **Brand kit ZIP**: extends Phase 3's `exporter/zip_export.rs` patterns.
- **PDF style guide**: reuses `jspdf` (already installed).

---

## Task Dependency Graph

```
T1 (logo_pipeline backend)     T3 (vectorizer backend)
  → T2 (Typography page +       → T4 (SVG editor + text controls)
      gallery + favorites)       
                                 T5 (brand_kit backend:
                                     resize + color variants)
                                   → T6 (style guide HTML)
                                     → T7 (ZIP export)
                                       → T8 (brand kit frontend + UI)
                                         
T9 (verification + Phase 7 commit)
```

Sequential on main for file conflicts. Backend tasks (T1, T3, T5) can land in any order; frontend tasks depend on their respective backend.

---

## Task 1: logo_pipeline backend

**Files:**
- Create: `src-tauri/src/logo_pipeline/{mod,types,pipeline,stub,commands}.rs`
- Create: `src-tauri/tests/logo_pipeline_integration.rs`
- Modify: `src-tauri/src/lib.rs`

### Approach

Mirrors `image_pipeline` but routes to `TaskKind::Logo` (→ Ideogram v3 per router from Phase 2). Variants feature parity with image_pipeline: N parallel calls with different seeds, all returning URLs.

### Steps

- [ ] **Step 1: Read existing image_pipeline for pattern parity**

```bash
cat src-tauri/src/image_pipeline/types.rs
cat src-tauri/src/image_pipeline/pipeline.rs | head -80
```

- [ ] **Step 2: types.rs**

```rust
//! Types for logo generation via Ideogram v3.

use std::path::PathBuf;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LogoStyle {
    #[default]
    Minimalist,
    Wordmark,
    Emblem,
    Mascot,
}

impl LogoStyle {
    pub fn brief(&self) -> &'static str {
        match self {
            Self::Minimalist => "minimalist, clean geometry, negative space, single color emphasis",
            Self::Wordmark => "wordmark style, bold custom typography, letterforms as the central visual",
            Self::Emblem => "emblem or badge style, circular/shield frame, contained visual hierarchy",
            Self::Mascot => "mascot character, friendly figure, expressive features",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogoInput {
    pub prompt: String,
    #[serde(default)]
    pub style: LogoStyle,
    #[serde(default = "default_count")]
    pub count: u32,
    #[serde(default)]
    pub palette: Option<String>,
    #[serde(default = "default_module")]
    pub module: String,
}

fn default_count() -> u32 { 5 }
fn default_module() -> String { "typography".to_string() }

#[derive(Debug, Clone, Serialize)]
pub struct LogoVariant {
    pub url: String,
    pub local_path: Option<PathBuf>,
    pub seed: Option<u32>,
    pub model: String,
}

#[derive(Debug, Error)]
pub enum LogoPipelineError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("router error: {0}")]
    Router(String),
    #[error("provider returned no image URL")]
    NoOutput,
    #[error("download failed: {0}")]
    Download(String),
    #[error("cache error: {0}")]
    Cache(String),
}

#[async_trait]
pub trait LogoPipeline: Send + Sync {
    async fn generate_variants(&self, input: LogoInput) -> Result<Vec<LogoVariant>, LogoPipelineError>;
}
```

- [ ] **Step 3: pipeline.rs + stub.rs + commands.rs**

Follow `image_pipeline` patterns verbatim. Key differences:
- Task: `TaskKind::Logo`
- Prompt construction: `"{prompt}. Style: {style.brief()}. Palette: {palette}"`
- Count defaults to 5; cap at 10
- Parallel generation via `futures::join_all`
- Cache: `<cache>/terryblemachine/logos/<sha256>.png`
- file:// special-case for tests

`generate_variants` signature returns `Vec<LogoVariant>`.

Add `StubLogoPipeline` returning deterministic stub URLs (matches other Stub patterns).

Commands.rs: `LogoPipelineState`, `LogoIpcError` (kebab-case), `#[tauri::command] generate_logo_variants`.

- [ ] **Step 4: Integration tests**

`src-tauri/tests/logo_pipeline_integration.rs`:
- `generates_variants_via_ideogram_route` — StubIdeogram returning canned URL, assert 5 variants
- `generate_variants_rejects_empty_prompt`
- `generate_variants_caps_count_at_10` — pass count=20, assert returns 10
- `download_is_idempotent`

Use `StubIdeogram` AiClient mirror pattern from `image_pipeline_integration.rs`.

- [ ] **Step 5: Register in lib.rs**

```rust
pub mod logo_pipeline;

// In setup:
let logo: Arc<dyn logo_pipeline::LogoPipeline> = Arc::new(
    logo_pipeline::RouterLogoPipeline::new(Arc::clone(&ai_router_for_setup))
);
app.manage(logo_pipeline::commands::LogoPipelineState::new(logo));
```

Add `logo_pipeline::commands::generate_logo_variants` to `invoke_handler!`.

- [ ] **Step 6: Verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -3 && cargo test 2>&1 | tail -5
cd .. && pnpm biome check . 2>&1 | tail -3
```

Commit:
```
feat(typography): logo_pipeline backend (Ideogram v3 variants)

- logo_pipeline module mirrors image_pipeline structure
- LogoStyle enum (Minimalist/Wordmark/Emblem/Mascot) feeds into prompt
- RouterLogoPipeline parallelizes N calls via futures::join_all
- Tauri command: generate_logo_variants
- 4 integration tests

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

Push + `gh run watch`.

---

## Task 2: Typography page scaffold + gallery

**Files:**
- Create: `src/lib/logoCommands.ts`
- Create: `src/pages/Typography.tsx`
- Create: `src/pages/Typography.test.tsx`
- Create: `src/components/typography/LogoGallery.tsx`
- Create: `src/components/typography/LogoGallery.test.tsx`
- Create: `src/stores/logoStore.ts` — favorites tracking
- Create: `src/stores/logoStore.test.ts`
- Modify: `src/App.tsx` — `/typography` route → `<TypographyPage />`
- Modify: `src/App.test.tsx` — drop typography from placeholder test

### Approach

Gallery is a 5-variant grid with per-card heart/favorite toggle. Favorites persist in Zustand store keyed by `variant.url`. Below the gallery, an Editor panel opens when a variant is selected.

### Steps

- [ ] **Step 1: Frontend wrapper**

```ts
// src/lib/logoCommands.ts
import { invoke } from "@tauri-apps/api/core";

export type LogoStyle = "minimalist" | "wordmark" | "emblem" | "mascot";

export interface LogoInput {
  prompt: string;
  style?: LogoStyle;
  count?: number;
  palette?: string;
  module?: string;
}

export interface LogoVariant {
  url: string;
  local_path: string | null;
  seed: number | null;
  model: string;
}

export const generateLogoVariants = (input: LogoInput) =>
  invoke<LogoVariant[]>("generate_logo_variants", { input });
```

- [ ] **Step 2: logoStore.ts**

```ts
// src/stores/logoStore.ts
import { create } from "zustand";

interface LogoState {
  favorites: Set<string>; // variant.url
  toggleFavorite: (url: string) => void;
  isFavorite: (url: string) => boolean;
  clearFavorites: () => void;
}

export const useLogoStore = create<LogoState>((set, get) => ({
  favorites: new Set(),
  toggleFavorite: (url) => set((state) => {
    const next = new Set(state.favorites);
    if (next.has(url)) next.delete(url);
    else next.add(url);
    return { favorites: next };
  }),
  isFavorite: (url) => get().favorites.has(url),
  clearFavorites: () => set({ favorites: new Set() }),
}));
```

Test:

```ts
// src/stores/logoStore.test.ts
import { beforeEach, describe, expect, it } from "vitest";
import { useLogoStore } from "@/stores/logoStore";

describe("logoStore", () => {
  beforeEach(() => useLogoStore.getState().clearFavorites());

  it("toggleFavorite adds then removes", () => {
    useLogoStore.getState().toggleFavorite("u1");
    expect(useLogoStore.getState().isFavorite("u1")).toBe(true);
    useLogoStore.getState().toggleFavorite("u1");
    expect(useLogoStore.getState().isFavorite("u1")).toBe(false);
  });

  it("isFavorite returns false for unknown url", () => {
    expect(useLogoStore.getState().isFavorite("nope")).toBe(false);
  });

  it("clearFavorites empties the set", () => {
    useLogoStore.getState().toggleFavorite("u1");
    useLogoStore.getState().toggleFavorite("u2");
    useLogoStore.getState().clearFavorites();
    expect(useLogoStore.getState().isFavorite("u1")).toBe(false);
    expect(useLogoStore.getState().isFavorite("u2")).toBe(false);
  });
});
```

- [ ] **Step 3: LogoGallery component + test**

```tsx
// src/components/typography/LogoGallery.tsx
import { Heart } from "lucide-react";
import type { LogoVariant } from "@/lib/logoCommands";
import { useLogoStore } from "@/stores/logoStore";

export interface LogoGalleryProps {
  variants: LogoVariant[];
  selectedUrl: string | null;
  onSelect: (url: string) => void;
}

export function LogoGallery({ variants, selectedUrl, onSelect }: LogoGalleryProps) {
  const isFavorite = useLogoStore((s) => s.isFavorite);
  const toggleFavorite = useLogoStore((s) => s.toggleFavorite);
  if (variants.length === 0) {
    return (
      <div className="flex h-full items-center justify-center font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
        No logos yet — generate variants above
      </div>
    );
  }
  return (
    <div className="grid grid-cols-3 gap-2 p-3">
      {variants.map((v) => {
        const fav = isFavorite(v.url);
        const selected = selectedUrl === v.url;
        return (
          <div
            key={v.url}
            className={`relative rounded-xs border ${selected ? "border-accent-500" : "border-neutral-dark-700"} bg-neutral-dark-900`}
            data-testid={`logo-variant-${v.url}`}
          >
            <button
              type="button"
              onClick={() => onSelect(v.url)}
              className="block aspect-square w-full overflow-hidden"
              aria-label={`Select logo variant`}
            >
              <img src={v.url} alt="" className="h-full w-full object-contain" />
            </button>
            <button
              type="button"
              onClick={() => toggleFavorite(v.url)}
              aria-label={fav ? "Unfavorite" : "Favorite"}
              className="absolute right-1 top-1 rounded-full bg-neutral-dark-950/80 p-1"
            >
              <Heart className={`h-3 w-3 ${fav ? "fill-accent-500 text-accent-500" : "text-neutral-dark-400"}`} />
            </button>
          </div>
        );
      })}
    </div>
  );
}
```

Test:

```tsx
// src/components/typography/LogoGallery.test.tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { LogoGallery } from "@/components/typography/LogoGallery";
import { useLogoStore } from "@/stores/logoStore";
import type { LogoVariant } from "@/lib/logoCommands";

const sample: LogoVariant[] = [
  { url: "u1", local_path: null, seed: null, model: "IdeogramV3" },
  { url: "u2", local_path: null, seed: null, model: "IdeogramV3" },
];

describe("LogoGallery", () => {
  beforeEach(() => useLogoStore.getState().clearFavorites());

  it("shows empty state when no variants", () => {
    render(<LogoGallery variants={[]} selectedUrl={null} onSelect={() => {}} />);
    expect(screen.getByText(/No logos yet/i)).toBeInTheDocument();
  });

  it("renders each variant", () => {
    render(<LogoGallery variants={sample} selectedUrl={null} onSelect={() => {}} />);
    expect(screen.getByTestId("logo-variant-u1")).toBeInTheDocument();
    expect(screen.getByTestId("logo-variant-u2")).toBeInTheDocument();
  });

  it("onSelect called when variant clicked", () => {
    const onSelect = vi.fn();
    render(<LogoGallery variants={sample} selectedUrl={null} onSelect={onSelect} />);
    fireEvent.click(screen.getAllByLabelText(/Select logo variant/)[0]);
    expect(onSelect).toHaveBeenCalledWith("u1");
  });

  it("toggleFavorite updates store", () => {
    render(<LogoGallery variants={sample} selectedUrl={null} onSelect={() => {}} />);
    fireEvent.click(screen.getAllByLabelText(/Favorite/)[0]);
    expect(useLogoStore.getState().isFavorite("u1")).toBe(true);
  });
});
```

- [ ] **Step 4: TypographyPage + test**

`src/pages/Typography.tsx`:

```tsx
import { Sparkles } from "lucide-react";
import { useState } from "react";
import { LogoGallery } from "@/components/typography/LogoGallery";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { generateLogoVariants, type LogoStyle, type LogoVariant } from "@/lib/logoCommands";
import { useUiStore } from "@/stores/uiStore";

export function TypographyPage() {
  const [prompt, setPrompt] = useState("");
  const [style, setStyle] = useState<LogoStyle>("minimalist");
  const [palette, setPalette] = useState("");
  const [busy, setBusy] = useState(false);
  const [variants, setVariants] = useState<LogoVariant[]>([]);
  const [selectedUrl, setSelectedUrl] = useState<string | null>(null);
  const notify = useUiStore((s) => s.notify);

  async function handleGenerate() {
    if (!prompt.trim()) return;
    setBusy(true);
    try {
      const results = await generateLogoVariants({
        prompt: prompt.trim(),
        style,
        count: 6,
        palette: palette.trim() || undefined,
        module: "typography",
      });
      setVariants(results);
      notify({ kind: "success", message: `Generated ${results.length} variants` });
    } catch (err) {
      notify({
        kind: "error",
        message: "Logo generation failed",
        detail: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="grid h-full grid-rows-[auto_1fr]">
      <div className="flex flex-col gap-3 border-neutral-dark-700 border-b p-6">
        <div className="flex items-center gap-2">
          <span className="font-mono text-2xs text-accent-500 uppercase tracking-label-wide">
            MOD—05 · TYPE & LOGO
          </span>
        </div>
        <div className="flex items-end gap-2">
          <div className="flex-1">
            <Input
              label="Describe the logo"
              id="logo-prompt"
              placeholder='"TERRYBLEMACHINE" — AI design tool, bold mark'
              value={prompt}
              onValueChange={setPrompt}
            />
          </div>
          <label className="flex flex-col gap-1">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Style
            </span>
            <select
              aria-label="Logo style"
              value={style}
              onChange={(e) => setStyle(e.target.value as LogoStyle)}
              className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 text-xs text-neutral-dark-100"
            >
              <option value="minimalist">Minimalist</option>
              <option value="wordmark">Wordmark</option>
              <option value="emblem">Emblem</option>
              <option value="mascot">Mascot</option>
            </select>
          </label>
          <div className="w-48">
            <Input
              label="Palette"
              id="logo-palette"
              placeholder="monochrome / warm / sunset"
              value={palette}
              onValueChange={setPalette}
            />
          </div>
          <Button variant="primary" onClick={handleGenerate} disabled={!prompt.trim() || busy}>
            <Sparkles className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
            {busy ? "Generating…" : "Generate 6 variants"}
          </Button>
        </div>
      </div>

      <div className="grid min-h-0 grid-cols-[1fr_18rem]">
        <LogoGallery variants={variants} selectedUrl={selectedUrl} onSelect={setSelectedUrl} />
        <div className="flex flex-col border-neutral-dark-700 border-l p-3">
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            Editor
          </span>
          {selectedUrl ? (
            <div className="mt-2 font-mono text-2xs text-neutral-dark-500">
              Selected · {selectedUrl.slice(-24)}
            </div>
          ) : (
            <div className="mt-2 font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
              No selection
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
```

Test mirrors Video.test.tsx (mock `logoCommands`, render, assert banner/input/gallery empty state).

- [ ] **Step 5: Route wiring + App.test.tsx drop**

`src/App.tsx`: `<Route path="/typography" element={<TypographyPage />} />` + import.

`src/App.test.tsx`: remove `["/typography", "Type & Logo", "typography"]` from `.each`.

Check `useModuleRouteSync.test.tsx` — if it asserts "Coming soon — Type & Logo", update to `/MOD—05 · TYPE & LOGO/`.

- [ ] **Step 6: Verify + commit**

Commit: `feat(typography): Typography page scaffold + LogoGallery with favorites`.

---

## Task 3: Vectorizer backend (VTracer)

**Files:**
- Modify: `src-tauri/Cargo.toml` — add `vtracer = "0.6"`
- Create: `src-tauri/src/vectorizer/{mod,types,pipeline,stub,commands}.rs`
- Create: `src-tauri/tests/vectorizer_integration.rs`
- Modify: `src-tauri/src/lib.rs`

### Approach

VTracer reads a raster (PNG/JPEG from a local path) and emits an SVG. Since logo variants are cached by `logo_pipeline` as local PNGs, vectorizer takes `local_path: PathBuf` input and returns `{ svg: String }`.

### Steps

- [ ] **Step 1: Add vtracer dep**

```bash
cd src-tauri && cargo add vtracer
```

Verify: `grep vtracer Cargo.toml`. Should show `vtracer = "0.6.x"` or similar.

If `vtracer` is incompatible with current Rust edition, try `visioncortex_vtracer` or find the equivalent. Report findings.

- [ ] **Step 2: types.rs**

```rust
use std::path::PathBuf;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct VectorizeInput {
    pub image_path: PathBuf,
    #[serde(default = "default_color_mode")]
    pub color_mode: String,  // "color" | "bw"
    #[serde(default = "default_filter_speckle")]
    pub filter_speckle: u32,
    #[serde(default = "default_corner_threshold")]
    pub corner_threshold: u32,
}
fn default_color_mode() -> String { "color".into() }
fn default_filter_speckle() -> u32 { 4 }
fn default_corner_threshold() -> u32 { 60 }

#[derive(Debug, Clone, Serialize)]
pub struct VectorizeResult {
    pub svg: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Error)]
pub enum VectorizeError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("vtracer error: {0}")]
    Vtracer(String),
    #[error("io error: {0}")]
    Io(String),
}

#[async_trait]
pub trait Vectorizer: Send + Sync {
    async fn vectorize(&self, input: VectorizeInput) -> Result<VectorizeResult, VectorizeError>;
}
```

- [ ] **Step 3: pipeline.rs — VTracer wrapper**

```rust
use std::fs;
use std::io::Write;
use async_trait::async_trait;
use tempfile::NamedTempFile;

use super::types::{VectorizeError, VectorizeInput, VectorizeResult, Vectorizer};

pub struct VtracerPipeline;
impl VtracerPipeline { pub fn new() -> Self { Self } }

#[async_trait]
impl Vectorizer for VtracerPipeline {
    async fn vectorize(&self, input: VectorizeInput) -> Result<VectorizeResult, VectorizeError> {
        if !input.image_path.exists() {
            return Err(VectorizeError::InvalidInput(format!(
                "image not found: {}",
                input.image_path.display()
            )));
        }

        // vtracer's API: vtracer::convert_image_to_svg(input_path, output_path, config) -> Result
        // Or: vtracer::convert(input_file, ColorMode, Hierarchical, ...) -> Result<String, Error>
        //
        // Check the crate's public API — the exact function name varies by version.
        // Most recent stable: `vtracer::convert_image_to_svg`.

        let output_file = NamedTempFile::new().map_err(|e| VectorizeError::Io(e.to_string()))?;
        let output_path = output_file.path().to_owned();

        let config = vtracer::Config {
            color_mode: match input.color_mode.as_str() {
                "bw" => vtracer::ColorMode::Binary,
                _ => vtracer::ColorMode::Color,
            },
            filter_speckle: input.filter_speckle as usize,
            corner_threshold: input.corner_threshold as u32,
            ..vtracer::Config::default()
        };

        // vtracer runs synchronous CPU work; spawn_blocking for Tokio cleanliness.
        let input_path = input.image_path.clone();
        let output_path_clone = output_path.clone();
        tokio::task::spawn_blocking(move || {
            vtracer::convert_image_to_svg(
                input_path.to_string_lossy().as_ref(),
                output_path_clone.to_string_lossy().as_ref(),
                config,
            )
            .map_err(|e| VectorizeError::Vtracer(format!("{e:?}")))
        })
        .await
        .map_err(|e| VectorizeError::Vtracer(format!("join error: {e}")))??;

        let svg = fs::read_to_string(&output_path)
            .map_err(|e| VectorizeError::Io(e.to_string()))?;

        // Parse width/height from SVG viewBox / width+height attributes
        let (width, height) = parse_svg_dimensions(&svg).unwrap_or((1024, 1024));

        Ok(VectorizeResult { svg, width, height })
    }
}

fn parse_svg_dimensions(svg: &str) -> Option<(u32, u32)> {
    // Simple regex-free scan: look for width="..." and height="..."
    let w = find_attr(svg, "width")?;
    let h = find_attr(svg, "height")?;
    Some((w, h))
}

fn find_attr(s: &str, attr: &str) -> Option<u32> {
    let needle = format!("{attr}=\"");
    let start = s.find(&needle)? + needle.len();
    let end = s[start..].find('"')?;
    s[start..start + end].parse::<u32>().ok()
}
```

If `vtracer`'s actual API signature differs, adjust. The convert function is the main one — read the docs with `cargo doc -p vtracer --open` or grep src.

- [ ] **Step 4: stub.rs**

```rust
use async_trait::async_trait;
use super::types::{VectorizeError, VectorizeInput, VectorizeResult, Vectorizer};

pub struct StubVectorizer;
impl StubVectorizer { pub fn new() -> Self { Self } }

#[async_trait]
impl Vectorizer for StubVectorizer {
    async fn vectorize(&self, input: VectorizeInput) -> Result<VectorizeResult, VectorizeError> {
        if !input.image_path.exists() {
            return Err(VectorizeError::InvalidInput("missing image".into()));
        }
        Ok(VectorizeResult {
            svg: format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"100\"><rect width=\"100\" height=\"100\" fill=\"#e85d2d\"/></svg>"
            ),
            width: 100,
            height: 100,
        })
    }
}
```

- [ ] **Step 5: commands.rs**

Mirror `logo_pipeline/commands.rs` pattern. `VectorizerState`, `VectorizeIpcError`, `#[tauri::command] vectorize_image`.

- [ ] **Step 6: mod.rs + integration tests**

```rust
// mod.rs
pub mod commands;
pub mod pipeline;
pub mod stub;
pub mod types;

pub use pipeline::VtracerPipeline;
pub use stub::StubVectorizer;
pub use types::{VectorizeError, VectorizeInput, VectorizeResult, Vectorizer};
```

Integration tests in `src-tauri/tests/vectorizer_integration.rs`:

```rust
use std::sync::Arc;
use tempfile::TempDir;
use terryblemachine_lib::vectorizer::{StubVectorizer, Vectorizer, VectorizeInput};

#[tokio::test]
async fn stub_vectorizer_returns_svg_for_existing_file() {
    let tmp = TempDir::new().unwrap();
    let img = tmp.path().join("x.png");
    std::fs::write(&img, b"fake-png").unwrap();

    let v = StubVectorizer::new();
    let result = v.vectorize(VectorizeInput {
        image_path: img,
        color_mode: "color".into(),
        filter_speckle: 4,
        corner_threshold: 60,
    }).await.unwrap();
    assert!(result.svg.contains("<svg"));
    assert!(result.width > 0);
}

#[tokio::test]
async fn stub_vectorizer_rejects_missing_file() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("nope.png");
    let v = StubVectorizer::new();
    let err = v.vectorize(VectorizeInput {
        image_path: missing,
        color_mode: "color".into(),
        filter_speckle: 4,
        corner_threshold: 60,
    }).await.unwrap_err();
    assert!(matches!(err, terryblemachine_lib::vectorizer::VectorizeError::InvalidInput(_)));
}
```

VtracerPipeline happy-path test would require a real raster. Skip with `#[ignore]` for manual verification, or write a tiny 4x4 PNG fixture bundled in tests.

- [ ] **Step 7: Register in lib.rs**

```rust
pub mod vectorizer;

// In setup:
let vect: Arc<dyn vectorizer::Vectorizer> = Arc::new(vectorizer::VtracerPipeline::new());
app.manage(vectorizer::commands::VectorizerState::new(vect));
```

Add `vectorizer::commands::vectorize_image` to `invoke_handler!`.

- [ ] **Step 8: Verify + commit**

```bash
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings 2>&1 | tail -3 && cargo test 2>&1 | tail -5
cd .. && pnpm biome check . 2>&1 | tail -3
```

Commit: `feat(typography): vectorizer backend (vtracer raster→SVG)`.

---

## Task 4: SVG editor + text controls

**Files:**
- Create: `src/lib/vectorizerCommands.ts`
- Create: `src/components/typography/SvgEditor.tsx`
- Create: `src/components/typography/SvgEditor.test.tsx`
- Create: `src/components/typography/TextLogoControls.tsx`
- Modify: `src/pages/Typography.tsx` — mount editor when variant selected

### Approach

Reuse Fabric.js via drei-pattern: a `<canvas>` with a Fabric instance loaded with the SVG (via `fabric.loadSVGFromString`). Text logos are edited via Textbox with Font/Color/Size/Kerning/Tracking controls.

Since Phase 4's `FabricCanvas` already has most infrastructure, create a separate lighter `SvgEditor` rather than overloading FabricCanvas (keeps concerns separate — graphic2d vs typography).

### Steps

- [ ] **Step 1: Frontend wrapper**

```ts
// src/lib/vectorizerCommands.ts
import { invoke } from "@tauri-apps/api/core";

export interface VectorizeInput {
  image_path: string;
  color_mode?: "color" | "bw";
  filter_speckle?: number;
  corner_threshold?: number;
}

export interface VectorizeResult {
  svg: string;
  width: number;
  height: number;
}

export const vectorizeImage = (input: VectorizeInput) =>
  invoke<VectorizeResult>("vectorize_image", { input });
```

- [ ] **Step 2: SvgEditor component**

```tsx
// src/components/typography/SvgEditor.tsx
import * as fabric from "fabric";
import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";

export interface SvgEditorHandle {
  canvas: () => fabric.Canvas | null;
  loadSvg: (svg: string, width: number, height: number) => Promise<void>;
  toSvgString: () => string;
}

export interface SvgEditorProps {
  className?: string;
}

export const SvgEditor = forwardRef<SvgEditorHandle, SvgEditorProps>(
  function SvgEditorImpl({ className }, ref) {
    const canvasElRef = useRef<HTMLCanvasElement | null>(null);
    const canvasRef = useRef<fabric.Canvas | null>(null);

    useEffect(() => {
      if (!canvasElRef.current) return;
      const c = new fabric.Canvas(canvasElRef.current, {
        width: 600,
        height: 400,
        backgroundColor: "#F7F7F8",
        preserveObjectStacking: true,
      });
      canvasRef.current = c;
      return () => {
        c.dispose();
        canvasRef.current = null;
      };
    }, []);

    useImperativeHandle(
      ref,
      (): SvgEditorHandle => ({
        canvas: () => canvasRef.current,
        async loadSvg(svg, width, height) {
          const c = canvasRef.current;
          if (!c) return;
          c.clear();
          c.setDimensions({ width, height });
          c.backgroundColor = "#F7F7F8";
          const result = await fabric.loadSVGFromString(svg);
          const group = new fabric.Group(result.objects as fabric.Object[], {
            left: 0,
            top: 0,
          });
          c.add(group);
          c.setActiveObject(group);
          c.requestRenderAll();
        },
        toSvgString() {
          const c = canvasRef.current;
          return c?.toSVG() ?? "";
        },
      }),
      [],
    );

    return (
      <div className={`flex h-full w-full items-center justify-center bg-neutral-dark-950 ${className ?? ""}`}>
        <canvas
          ref={canvasElRef}
          data-testid="svg-editor-canvas"
          className="rounded-xs border border-neutral-dark-700"
        />
      </div>
    );
  },
);
```

- [ ] **Step 3: SvgEditor test**

Test uses vitest-canvas-mock (already installed T17) to run Fabric.Canvas in jsdom:

```tsx
// src/components/typography/SvgEditor.test.tsx
import { render } from "@testing-library/react";
import { createRef } from "react";
import { describe, expect, it } from "vitest";
import { SvgEditor, type SvgEditorHandle } from "@/components/typography/SvgEditor";

describe("SvgEditor", () => {
  it("exposes handle methods", () => {
    const ref = createRef<SvgEditorHandle>();
    render(<SvgEditor ref={ref} />);
    expect(typeof ref.current?.loadSvg).toBe("function");
    expect(typeof ref.current?.toSvgString).toBe("function");
    expect(ref.current?.canvas()).not.toBeNull();
  });

  it("loadSvg + toSvgString roundtrip", async () => {
    const ref = createRef<SvgEditorHandle>();
    render(<SvgEditor ref={ref} />);
    const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect width="100" height="100" fill="#e85d2d"/></svg>';
    await ref.current!.loadSvg(svg, 100, 100);
    const out = ref.current!.toSvgString();
    expect(out).toContain("<svg");
    // Fabric re-emits SVG, should contain an element representing the rect
    expect(out.length).toBeGreaterThan(50);
  });
});
```

If fabric.loadSVGFromString behaves oddly under canvas-mock, gate the 2nd test with try/catch and document.

- [ ] **Step 4: TextLogoControls**

```tsx
// src/components/typography/TextLogoControls.tsx
import { GOOGLE_FONTS, injectGoogleFont, type GoogleFont } from "@/lib/googleFonts";

export interface TextStyle {
  font: string;
  color: string;
  size: number;
  kerning: number; // letter-spacing in px
  tracking: number; // word-spacing in px
}

export interface TextLogoControlsProps {
  value: TextStyle;
  onChange: (next: TextStyle) => void;
}

export function TextLogoControls({ value, onChange }: TextLogoControlsProps) {
  return (
    <div className="flex flex-col gap-2">
      <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
        Text
      </span>
      <select
        aria-label="Font"
        value={value.font}
        onChange={async (e) => {
          const next = e.target.value as GoogleFont;
          await injectGoogleFont(next);
          onChange({ ...value, font: next });
        }}
        className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 text-xs text-neutral-dark-100"
      >
        {GOOGLE_FONTS.map((f) => <option key={f} value={f}>{f}</option>)}
      </select>
      <label className="flex items-center gap-2 text-2xs text-neutral-dark-300">
        Color
        <input
          aria-label="Color"
          type="color"
          value={value.color}
          onChange={(e) => onChange({ ...value, color: e.target.value })}
          className="h-6 w-10 cursor-pointer"
        />
      </label>
      <label className="flex flex-col gap-1 text-2xs text-neutral-dark-300">
        Size: {value.size}px
        <input
          aria-label="Size"
          type="range"
          min={12}
          max={240}
          step={1}
          value={value.size}
          onChange={(e) => onChange({ ...value, size: Number(e.target.value) })}
          className="accent-accent-500"
        />
      </label>
      <label className="flex flex-col gap-1 text-2xs text-neutral-dark-300">
        Kerning: {value.kerning.toFixed(1)}
        <input
          aria-label="Kerning"
          type="range"
          min={-5}
          max={30}
          step={0.5}
          value={value.kerning}
          onChange={(e) => onChange({ ...value, kerning: Number(e.target.value) })}
          className="accent-accent-500"
        />
      </label>
      <label className="flex flex-col gap-1 text-2xs text-neutral-dark-300">
        Tracking: {value.tracking.toFixed(1)}
        <input
          aria-label="Tracking"
          type="range"
          min={0}
          max={50}
          step={0.5}
          value={value.tracking}
          onChange={(e) => onChange({ ...value, tracking: Number(e.target.value) })}
          className="accent-accent-500"
        />
      </label>
    </div>
  );
}
```

Test: render, changing each control fires onChange with the correct patch.

- [ ] **Step 5: Wire into Typography.tsx**

When a variant is selected, display SvgEditor. Add a "Vectorize" button next to the selected variant that calls `vectorizeImage({ image_path: variant.local_path })`, then `editor.loadSvg(result.svg, result.width, result.height)`. TextLogoControls only appears when the editor has a text object (for now, just expose the button always — text edit wiring minimal).

For this task, the minimum is: "Vectorize" button + SvgEditor renders SVG + TextLogoControls panel. Full text-to-SVG + apply-kerning-to-fabric-textbox is deferred to polish.

- [ ] **Step 6: Verify + commit**

Commit: `feat(typography): SvgEditor + TextLogoControls + vectorize flow`.

---

## Task 5: Brand kit resize + color variants (backend)

**Files:**
- Modify: `src-tauri/Cargo.toml` — add `image = "0.25"` (or check if already present)
- Create: `src-tauri/src/brand_kit/{mod,types,pipeline,commands}.rs`
- Create: `src-tauri/tests/brand_kit_integration.rs`
- Modify: `src-tauri/src/lib.rs`

### Approach

Given a source SVG string + source raster PNG, produce:
- Sizes: 16/32/64 (favicon), 128/256/512 (web), 1024/2048 (print)
- Color variants: original (pass-through), B&W, inverted

Use `image` crate for raster resizing + color manipulation. SVG stays as-is for vector variants; raster color variants derived from the source PNG.

### Steps

- [ ] **Step 1: Check image crate**

```bash
grep 'image = ' src-tauri/Cargo.toml
```

If present, skip install. If not: `cargo add image`. Check version is 0.25+.

- [ ] **Step 2: types.rs**

```rust
use std::path::PathBuf;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct BrandKitInput {
    pub logo_svg: String,
    pub source_png_path: PathBuf,
    pub brand_name: String,
    pub primary_color: String,  // hex
    pub accent_color: String,
    pub font: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrandKitAsset {
    pub filename: String,
    pub bytes: Vec<u8>,  // file content
}

#[derive(Debug, Clone, Serialize)]
pub struct BrandKitResult {
    pub assets: Vec<BrandKitAsset>,
    pub style_guide_html: String,
}

#[derive(Debug, Error)]
pub enum BrandKitError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("image error: {0}")]
    Image(String),
    #[error("io error: {0}")]
    Io(String),
}

#[async_trait]
pub trait BrandKitBuilder: Send + Sync {
    async fn build(&self, input: BrandKitInput) -> Result<BrandKitResult, BrandKitError>;
}
```

- [ ] **Step 3: pipeline.rs — resize + color variants**

```rust
use async_trait::async_trait;
use image::{GenericImageView, ImageBuffer, Rgba};

use super::types::{BrandKitAsset, BrandKitBuilder, BrandKitError, BrandKitInput, BrandKitResult};

pub struct StandardBrandKit;
impl StandardBrandKit { pub fn new() -> Self { Self } }

const SIZES: &[(u32, &str)] = &[
    (16, "favicon-16"),
    (32, "favicon-32"),
    (64, "favicon-64"),
    (128, "logo-128"),
    (256, "logo-256"),
    (512, "logo-512"),
    (1024, "print-1024"),
    (2048, "print-2048"),
];

#[async_trait]
impl BrandKitBuilder for StandardBrandKit {
    async fn build(&self, input: BrandKitInput) -> Result<BrandKitResult, BrandKitError> {
        if input.logo_svg.trim().is_empty() {
            return Err(BrandKitError::InvalidInput("logo_svg empty".into()));
        }
        if !input.source_png_path.exists() {
            return Err(BrandKitError::InvalidInput(format!(
                "source_png missing: {}", input.source_png_path.display()
            )));
        }
        let source_bytes = std::fs::read(&input.source_png_path)
            .map_err(|e| BrandKitError::Io(e.to_string()))?;
        let img = image::load_from_memory(&source_bytes)
            .map_err(|e| BrandKitError::Image(e.to_string()))?;

        let mut assets: Vec<BrandKitAsset> = Vec::new();

        // Pass-through SVG
        assets.push(BrandKitAsset {
            filename: "logo.svg".into(),
            bytes: input.logo_svg.as_bytes().to_vec(),
        });

        // Raster sizes — original palette
        for &(size, label) in SIZES {
            let resized = img.resize(size, size, image::imageops::FilterType::Lanczos3);
            let mut buf = Vec::new();
            let mut cursor = std::io::Cursor::new(&mut buf);
            resized.write_to(&mut cursor, image::ImageFormat::Png)
                .map_err(|e| BrandKitError::Image(e.to_string()))?;
            assets.push(BrandKitAsset {
                filename: format!("{label}.png"),
                bytes: buf,
            });
        }

        // B&W variant (grayscale)
        let bw = img.grayscale();
        let mut bw_buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut bw_buf);
        bw.write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| BrandKitError::Image(e.to_string()))?;
        assets.push(BrandKitAsset {
            filename: "logo-bw.png".into(),
            bytes: bw_buf,
        });

        // Inverted variant
        let mut inv = img.to_rgba8();
        for pixel in inv.pixels_mut() {
            pixel.0[0] = 255 - pixel.0[0];
            pixel.0[1] = 255 - pixel.0[1];
            pixel.0[2] = 255 - pixel.0[2];
        }
        let inverted: ImageBuffer<Rgba<u8>, Vec<u8>> = inv;
        let mut inv_buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut inv_buf);
        image::DynamicImage::ImageRgba8(inverted).write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| BrandKitError::Image(e.to_string()))?;
        assets.push(BrandKitAsset {
            filename: "logo-inverted.png".into(),
            bytes: inv_buf,
        });

        // Style guide built in Task 6; placeholder here
        let style_guide_html = super::style_guide::build_style_guide(&input);

        Ok(BrandKitResult { assets, style_guide_html })
    }
}
```

Note: T6 adds `brand_kit/style_guide.rs`. For T5, inline a placeholder: `fn build_style_guide(_: &BrandKitInput) -> String { "<html></html>".into() }`, to be replaced in T6.

- [ ] **Step 4: Integration test**

`src-tauri/tests/brand_kit_integration.rs`:

```rust
use std::sync::Arc;
use tempfile::TempDir;
use terryblemachine_lib::brand_kit::{BrandKitBuilder, BrandKitInput, StandardBrandKit};

fn tiny_png() -> Vec<u8> {
    // 2x2 PNG (red/green/blue/white checkered) — minimal valid PNG
    vec![
        0x89,0x50,0x4E,0x47,0x0D,0x0A,0x1A,0x0A,
        0x00,0x00,0x00,0x0D,0x49,0x48,0x44,0x52,
        0x00,0x00,0x00,0x02,0x00,0x00,0x00,0x02,
        0x08,0x02,0x00,0x00,0x00,0xFD,0xD4,0x9A,0x73,
        0x00,0x00,0x00,0x15,0x49,0x44,0x41,0x54,
        0x78,0x9C,0x62,0xFC,0xCF,0xC0,0xC0,0xC0,
        0xF0,0x9F,0xC1,0xC0,0xF0,0x1F,0x00,0x00,
        0xFF,0xFF,0x03,0x00,0x05,0xFE,0x02,0xFE,
        0xDC,0xCC,0x59,0xE7,
        0x00,0x00,0x00,0x00,0x49,0x45,0x4E,0x44,0xAE,0x42,0x60,0x82,
    ]
}

#[tokio::test]
async fn brand_kit_produces_all_sizes_plus_variants() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src.png");
    std::fs::write(&src, tiny_png()).unwrap();

    let kit = StandardBrandKit::new();
    let result = kit.build(BrandKitInput {
        logo_svg: "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"/>".into(),
        source_png_path: src,
        brand_name: "Acme".into(),
        primary_color: "#e85d2d".into(),
        accent_color: "#0E0E11".into(),
        font: "Inter".into(),
    }).await.unwrap();

    // 8 sizes + 1 svg + 1 bw + 1 inverted = 11 assets minimum
    assert!(result.assets.len() >= 11, "expected ≥11 assets, got {}", result.assets.len());
    let filenames: Vec<_> = result.assets.iter().map(|a| a.filename.as_str()).collect();
    assert!(filenames.contains(&"logo.svg"));
    assert!(filenames.contains(&"favicon-16.png"));
    assert!(filenames.contains(&"logo-bw.png"));
    assert!(filenames.contains(&"logo-inverted.png"));
}

#[tokio::test]
async fn brand_kit_rejects_empty_svg() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src.png");
    std::fs::write(&src, tiny_png()).unwrap();

    let kit = StandardBrandKit::new();
    let err = kit.build(BrandKitInput {
        logo_svg: "   ".into(),
        source_png_path: src,
        brand_name: "X".into(),
        primary_color: "#000".into(),
        accent_color: "#fff".into(),
        font: "Inter".into(),
    }).await.unwrap_err();
    assert!(matches!(err, terryblemachine_lib::brand_kit::BrandKitError::InvalidInput(_)));
}
```

If the inlined tiny_png bytes are invalid (I may have transcribed incorrectly), generate a real one inline via the `image` crate: create a 2x2 ImageBuffer and write to bytes.

- [ ] **Step 5: mod.rs, commands.rs, lib.rs registration**

Same pattern as other pipelines. Export all public types. `#[tauri::command] build_brand_kit`.

- [ ] **Step 6: Verify + commit**

Commit: `feat(typography): brand_kit resize + color variants`.

---

## Task 6: Style guide HTML generator

**Files:**
- Create: `src-tauri/src/brand_kit/style_guide.rs`
- Modify: `src-tauri/src/brand_kit/pipeline.rs` — call style_guide::build_style_guide

### Approach

Generate a clean HTML document embedding:
- Brand name + subtitle
- Logo (SVG inline)
- Palette swatches (HEX + RGB)
- Typography sample (font specimen)
- Spacing guide (min size rules)

Single HTML file, inline CSS. PDF conversion handled by caller via jspdf on frontend (or printf→PDF not covered in this task).

### Steps

- [ ] **Step 1: style_guide.rs**

```rust
use super::types::BrandKitInput;

pub fn build_style_guide(input: &BrandKitInput) -> String {
    let name = &input.brand_name;
    let primary = &input.primary_color;
    let accent = &input.accent_color;
    let font = &input.font;
    let svg = &input.logo_svg;

    format!(r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{name} · Brand Guidelines</title>
<style>
  body {{ margin: 0; font-family: "{font}", sans-serif; color: #0E0E11; background: #F7F7F8; }}
  .container {{ max-width: 960px; margin: 0 auto; padding: 64px 32px; }}
  h1 {{ font-size: 4rem; margin: 0 0 0.25em; }}
  h2 {{ font-size: 1.125rem; text-transform: uppercase; letter-spacing: 0.08em; margin: 2em 0 1em; color: #6b6b70; }}
  .logo {{ width: 240px; height: 240px; padding: 32px; background: white; border: 1px solid #e5e5e5; }}
  .palette {{ display: flex; gap: 16px; }}
  .swatch {{ width: 140px; }}
  .swatch .chip {{ width: 100%; aspect-ratio: 1; border: 1px solid #e5e5e5; }}
  .swatch .meta {{ font-family: ui-monospace, "IBM Plex Mono", monospace; font-size: 0.75rem; padding-top: 8px; }}
  .specimen {{ font-size: 6rem; line-height: 1; margin: 0; }}
  .rules {{ font-size: 0.875rem; line-height: 1.6; color: #4b4b50; }}
  .rules li {{ margin-bottom: 0.5em; }}
</style>
</head>
<body>
<div class="container">
  <h1>{name}</h1>
  <p>Brand guidelines — v1.0</p>

  <h2>Logo</h2>
  <div class="logo">{svg}</div>
  <ul class="rules">
    <li>Minimum size: 24px height on screen, 12mm in print.</li>
    <li>Keep clear space equal to the height of the mark around all sides.</li>
    <li>Do not rotate, stretch, or recolor outside the provided variants.</li>
  </ul>

  <h2>Palette</h2>
  <div class="palette">
    <div class="swatch">
      <div class="chip" style="background: {primary};"></div>
      <div class="meta">Primary<br>{primary}</div>
    </div>
    <div class="swatch">
      <div class="chip" style="background: {accent};"></div>
      <div class="meta">Accent<br>{accent}</div>
    </div>
  </div>

  <h2>Typography</h2>
  <p class="specimen">{name}</p>
  <p class="rules">Typeface: {font}. Use for display and UI body text.</p>
</div>
</body>
</html>"#)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::BrandKitInput;
    use std::path::PathBuf;

    #[test]
    fn style_guide_embeds_brand_name_and_palette() {
        let input = BrandKitInput {
            logo_svg: "<svg></svg>".into(),
            source_png_path: PathBuf::from("x.png"),
            brand_name: "Acme".into(),
            primary_color: "#e85d2d".into(),
            accent_color: "#0E0E11".into(),
            font: "Inter".into(),
        };
        let html = build_style_guide(&input);
        assert!(html.contains("Acme"));
        assert!(html.contains("#e85d2d"));
        assert!(html.contains("#0E0E11"));
        assert!(html.contains("Inter"));
        assert!(html.contains("<svg>"));
    }
}
```

- [ ] **Step 2: Wire into pipeline.rs**

Replace the placeholder with real call:
```rust
let style_guide_html = super::style_guide::build_style_guide(&input);
```

Also add `style_guide.html` as an asset in the returned list:
```rust
assets.push(BrandKitAsset {
    filename: "style-guide.html".into(),
    bytes: style_guide_html.as_bytes().to_vec(),
});
```

- [ ] **Step 3: Extend integration test**

Add to `brand_kit_integration.rs`:

```rust
#[tokio::test]
async fn brand_kit_includes_style_guide_html() {
    // ... same setup as above ...
    let filenames: Vec<_> = result.assets.iter().map(|a| a.filename.as_str()).collect();
    assert!(filenames.contains(&"style-guide.html"));
    let guide = result.assets.iter().find(|a| a.filename == "style-guide.html").unwrap();
    let html = String::from_utf8(guide.bytes.clone()).unwrap();
    assert!(html.contains("Acme"));
    assert!(html.contains("#e85d2d"));
}
```

- [ ] **Step 4: Verify + commit**

Commit: `feat(typography): brand kit style-guide HTML generator`.

---

## Task 7: Brand kit ZIP export

**Files:**
- Modify: `src-tauri/src/brand_kit/pipeline.rs` OR `src-tauri/src/brand_kit/export.rs` (new) — ZIP writer
- Modify: `src-tauri/src/brand_kit/commands.rs` — `export_brand_kit(destination_path)` Tauri command

### Approach

Reuse `zip` crate already used in Phase 3 exporter. Accept `destination_dir: PathBuf`, produce `<destination>/<brand-name-slug>-brand-kit.zip` containing all assets from `BrandKitResult`.

### Steps

- [ ] **Step 1: Export helper in brand_kit/**

Create `src-tauri/src/brand_kit/export.rs`:

```rust
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::types::{BrandKitAsset, BrandKitError};

pub fn write_zip(
    destination: &Path,
    brand_slug: &str,
    assets: &[BrandKitAsset],
) -> Result<PathBuf, BrandKitError> {
    std::fs::create_dir_all(destination).map_err(|e| BrandKitError::Io(e.to_string()))?;
    let path = destination.join(format!("{brand_slug}-brand-kit.zip"));
    let file = File::create(&path).map_err(|e| BrandKitError::Io(e.to_string()))?;
    let mut zip = ZipWriter::new(file);
    let options: SimpleFileOptions = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    for asset in assets {
        zip.start_file(&asset.filename, options)
            .map_err(|e| BrandKitError::Io(e.to_string()))?;
        zip.write_all(&asset.bytes).map_err(|e| BrandKitError::Io(e.to_string()))?;
    }
    zip.finish().map_err(|e| BrandKitError::Io(e.to_string()))?;
    Ok(path)
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut prev_hyphen = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_hyphen = false;
        } else if !prev_hyphen {
            out.push('-');
            prev_hyphen = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() { "brand".into() } else { trimmed }
}

pub fn slug_for(name: &str) -> String { slug(name) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slug_normalizes() {
        assert_eq!(slug("Hello World"), "hello-world");
        assert_eq!(slug("!!!"), "brand");
    }
}
```

- [ ] **Step 2: Tauri command**

In `commands.rs` add:

```rust
#[tauri::command]
pub async fn export_brand_kit(
    state: tauri::State<'_, BrandKitBuilderState>,
    input: BrandKitInput,
    destination: std::path::PathBuf,
) -> Result<std::path::PathBuf, BrandKitIpcError> {
    let result = state.0.build(input.clone()).await.map_err(Into::<BrandKitIpcError>::into)?;
    let slug = super::export::slug_for(&input.brand_name);
    super::export::write_zip(&destination, &slug, &result.assets)
        .map_err(Into::into)
}
```

Register in lib.rs.

- [ ] **Step 3: Integration test**

Add to `brand_kit_integration.rs`:

```rust
#[test]
fn zip_export_contains_all_assets() {
    use terryblemachine_lib::brand_kit::export::{slug_for, write_zip};
    use terryblemachine_lib::brand_kit::types::BrandKitAsset;
    use std::io::Read;

    let tmp = TempDir::new().unwrap();
    let assets = vec![
        BrandKitAsset { filename: "a.txt".into(), bytes: b"hello".to_vec() },
        BrandKitAsset { filename: "b.svg".into(), bytes: b"<svg/>".to_vec() },
    ];
    let path = write_zip(tmp.path(), &slug_for("Acme Brand"), &assets).unwrap();
    assert!(path.exists());
    assert!(path.file_name().unwrap().to_str().unwrap().contains("acme-brand"));

    let bytes = std::fs::read(&path).unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let names: Vec<String> = (0..archive.len()).map(|i| archive.by_index(i).unwrap().name().to_string()).collect();
    assert!(names.contains(&"a.txt".to_string()));
    assert!(names.contains(&"b.svg".to_string()));
    // Verify content
    let mut f = archive.by_name("a.txt").unwrap();
    let mut content = String::new();
    f.read_to_string(&mut content).unwrap();
    assert_eq!(content, "hello");
}
```

- [ ] **Step 4: Verify + commit**

Commit: `feat(typography): brand kit ZIP export`.

---

## Task 8: Brand kit frontend + export UI

**Files:**
- Create: `src/lib/brandKitCommands.ts`
- Create: `src/components/typography/BrandKitDialog.tsx`
- Create: `src/components/typography/BrandKitDialog.test.tsx`
- Modify: `src/pages/Typography.tsx` — Export button + dialog

### Steps

- [ ] **Step 1: Frontend wrapper**

```ts
// src/lib/brandKitCommands.ts
import { invoke } from "@tauri-apps/api/core";

export interface BrandKitInput {
  logo_svg: string;
  source_png_path: string;
  brand_name: string;
  primary_color: string;
  accent_color: string;
  font: string;
}

export const exportBrandKit = (input: BrandKitInput, destination: string) =>
  invoke<string>("export_brand_kit", { input, destination });
```

- [ ] **Step 2: BrandKitDialog component + test**

Modal with fields:
- brand name, primary color, accent color, font dropdown (from GOOGLE_FONTS), destination path input

Submit handler calls `exportBrandKit(...)` and notifies with resulting ZIP path.

- [ ] **Step 3: Wire into Typography.tsx**

Add "Export brand kit" button visible when variant is selected + svg is loaded in editor. On click, open dialog → export.

Use `currentProject.path/exports/` as destination default (mirror T14 pattern).

- [ ] **Step 4: Verify + commit**

Commit: `feat(typography): brand kit dialog + export wiring`.

---

## Task 9: Phase 7 verification + final commit

**Files:**
- Create: `docs/superpowers/specs/2026-04-17-phase-7-verification-report.md`

### Steps

- [ ] **Step 1: Full verify**

```bash
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd .. && pnpm exec tsc --noEmit && pnpm biome check . && pnpm test -- --run
```

- [ ] **Step 2: Write report**

Map each of 7.1/7.2/7.3 to closing commits. Document scope deferrals (system font enumeration, elaborate SVG path-level Bezier editing).

- [ ] **Step 3: Commit + push + CI**

```
feat(typography): Phase 7 abgeschlossen — Typografie & Logos

Closing: Logo generation via Ideogram v3 (7.1), vtracer vectorizer +
SVG editor + text controls (7.2), brand kit resize/color-variants/
style-guide/ZIP export (7.3).

9 Phase 7 tasks. 3 scope deferrals: elaborate Bezier path editing, local
system font enumeration, PDF style guide rendering — tracked as
follow-ups.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

Push + `gh run watch`.

---

## Self-review

**Spec coverage:**
- 7.1 Logo-Generation → T1 (backend) + T2 (UI + gallery + favorites)
- 7.2 Vektorisierung & SVG-Editor → T3 (vtracer backend) + T4 (SVG editor + text controls)
- 7.3 Brand-Asset-Export → T5 (resize + color variants) + T6 (style guide HTML) + T7 (ZIP) + T8 (frontend UI)

All 3 spec items covered. Bezier path-level editing is documented as deferred (minimum SVG edit capability via Fabric's group transform ships).

**Placeholder scan:** No "TBD". Deferrals are explicit with reason: system fonts need a Tauri plugin; Bezier polygon editing is rabbit-hole.

**Type consistency:**
- `LogoStyle` enum: TS + Rust match ("minimalist" | "wordmark" | "emblem" | "mascot")
- `LogoVariant` / `VectorizeResult` / `BrandKitInput` serde shape matches TS interfaces
- `BrandKitAsset` { filename, bytes } same both sides
- Tauri command names unique: `generate_logo_variants`, `vectorize_image`, `build_brand_kit`, `export_brand_kit`

**Risk areas:**
- `vtracer` crate API may require minor adaptation — plan explicitly says to check `cargo doc` if signature differs
- `fabric.loadSVGFromString` under vitest-canvas-mock may behave oddly; plan has graceful fallback
- Tiny PNG fixture in T5 test may need regeneration if bytes are wrong — plan flags this

---

**Plan complete and saved to `docs/superpowers/plans/2026-04-17-phase-7-typography.md`.**

Execution via `superpowers:subagent-driven-development`.
