# Phase 9 (Claude Subscription Bridge + Tool UX) — Verification Report

**Date:** 2026-04-19
**Spec:** `docs/superpowers/specs/2026-04-19-phase-9-claude-bridge-and-tool-ux-design.md`
**Plan:** `docs/superpowers/plans/2026-04-19-phase-9-claude-bridge-and-tool-ux.md`
**Sub-Project A closure note:** `docs/superpowers/specs/2026-04-19-phase-9-sub-a-verification.md` (T1-T7)

## Summary

Phase 9 shipped **two tightly-coupled feature blocks** before the first live test:

1. **Claude CLI Bridge** — all backend Claude calls can now route through the user's `claude` CLI (subscription billing, no API credits). Transport selectable in Settings (`auto` / `api` / `cli`); `auto` prefers CLI when detected on disk.
2. **Tool-selection UX** — per-prompt `ToolDropdown` + `/tool` override parser + `Optimize` toggle with Undo, wired into all five generative module pages (Graphic2D, WebsiteBuilder, Graphic3D, Video, Typography) plus the new Chat page. Optimize-then-dispatch order is fixed by design: `parseOverride` first → `optimize` the cleaned prompt via Claude → re-attach slug → dispatch.

**Counts:** cargo **475 passed / 0 failed / 4 ignored** across 12 binaries; vitest **471 passed / 0 failed** across 92 files; 2 new E2E specs green; biome clean (243 files, 0 issues).

## Sub-Project A — Claude CLI Bridge (T1-T7)

| Task | Commit | What shipped |
|---|---|---|
| T1 | 5cd4f4a | `detect_claude_binary()` with candidate paths (`which claude`, `~/.claude/local/claude`, `/opt/homebrew/bin/claude`, `/usr/local/bin/claude`) |
| T2 | 49c9167 | `StreamAccumulator` parses `claude -p --output-format stream-json` line-by-line; handles system / assistant / result events; 5 tests |
| T3 | d4bf2e0 | `ClaudeCliClient` implements `AiClient`; `Spawner` trait for test-time injection; `TokioSpawner` with `kill_on_drop(true)`; 4 mock tests |
| T4 | e374264 | 3 Tauri commands: `detect_claude_cli`, `get_claude_transport`, `set_claude_transport`; transport-aware `registry::build_default_clients` |
| T5 | adc2685 | Settings UI: Claude row transport tri-state (Auto / API / Subscription-CLI) with auto-detect display |
| T6 | d97b03b | `AiRequest.model_override: Option<Model>`; router honors override as primary while keeping strategy-defined fallbacks |
| T7 | 0f41113 | Sub-A closure note |

## Sub-Project B — Chat + Tool UX (T8-T22)

### Foundation (T8-T10, wave B1)
- `src/lib/toolCatalog.ts` — `TaskKind → ToolDef[]` capability matrix (13 TaskKinds × tier system)
- `src/lib/promptOverride.ts` — `parseOverride(text) → { override, cleanPrompt, slugLocation }`; whitelist via `OVERRIDE_ALIASES` (20+ slugs)
- `src/components/ui/ToolDropdown.tsx` — grouped-by-tier dropdown driven by the capability matrix

### Optimize backbone (T11-T13, wave B2 + quality follow-up f43d961)
- `optimize_prompt` IPC — 4 meta-templates (visual / code / logo / generic); returns `Result<String, RouterIpcError>`
- `useOptimizePrompt` hook — `optimize(inputOverride?) → Promise<string | undefined>`; `canUndo` + undo flow; concurrent-call guard
- `OptimizeToggle` UI — switch + `Loader2` spinner + Undo button; `aria-busy` + `disabled` when busy

### Chat panel scaffold (T14-T17, wave B3 + quality follow-up d1b4d7a)
- `chatStore` (Zustand + localStorage) — `addMessage` (returns id) / `appendChunk` / `newChat` / `hydrate`
- `chat_send_message` Tauri command — routes via `AiRouterState` (honors transport choice); emits `chat:stream-chunk` + `chat:stream-done`; returns `Result<(), RouterIpcError>`
- `sendChatMessage` TS wrapper — `doneFired` guard prevents double-fire even if Tauri panics
- `ChatPage` — 3-row grid (header / messages with `role="log" aria-live="polite"` / footer with ToolDropdown + OptimizeToggle + textarea + Send); Cmd+Enter sends; Send disabled when `!trim() || busy`
- Sidebar entry "Chat with Claude" above Settings; `Cmd+L` global shortcut

### Per-page integration (T18-T19, wave B4 + fix a644247)
Graphic2D prototype established the pattern (81eac14) and the design-spec-compliant order fix (a644247) nailed it: `parseOverride` → `optimize(cleanPrompt)` → re-attach slug → resolve finalModel → dispatch. Replicated across WebsiteBuilder (TextGeneration), Graphic3D (Text3D, text-only path), Video (TextToVideo, storyboard flow), Typography (Logo) in commit c6f8eba. `model_override: Option<Model>` threaded through: UI state → `/tool` override resolve → TS wrapper input type → Rust primary-input struct → `AiRequest.model_override`.

Rust modules extended: `image_pipeline`, `code_generator`, `mesh_pipeline`, `storyboard_generator`, `logo_pipeline`. Each gained one deser test asserting `model_override` round-trips from PascalCase JSON.

### Surfaces (T20-T21, wave B5)
- `ShortcutHelpOverlay` gained a "Tool overrides" section listing `OVERRIDE_ALIASES` (`/claude`, `/flux`, `/kling`, `/ideogram`, …); opens via `Cmd+/` or `?` (b03f165)
- E2E specs (27d0a59): `chat.spec.ts` smoke + `generate-with-override.spec.ts` asserts `/flux a sunset` dispatches `generate_variants` with `model_override: "FalFluxPro"` and cleanPrompt `"a sunset"`; Optimize toggle OFF → `optimize_prompt` never invoked

## Pillars (spec coverage)

| Pillar | Status | Tasks |
|---|---|---|
| `ClaudeCliClient` as drop-in `AiClient` | Done | T1-T3 |
| Transport selector + CLI detection (IPC + UI) | Done | T4-T5 |
| `AiRequest.model_override` + router integration | Done | T6 |
| Capability matrix + dropdown | Done | T8 + T10 |
| `/tool` override parser | Done | T9 |
| Optimize IPC + hook + toggle | Done | T11-T13 |
| Chat store + IPC + page + sidebar | Done | T14-T17 |
| Per-page wiring (5 modules) | Done | T18-T19 |
| Shortcut overlay extension | Done | T20 |
| E2E chat + override | Done | T21 |
| Closure report | Done | T22 (this) |

## Deviations from spec

Verified by reading `src-tauri/src/api_clients/claude_cli/chat.rs` and `src/pages/Chat.tsx` at HEAD (27d0a59):

- **T15 / chat single-shot streaming** — `chat_send_message` buffers the full Claude response and emits one `chat:stream-chunk` plus one `chat:stream-done` rather than token-by-token. The event contract is stable (`chat:stream-chunk` / `chat:stream-done`); upgrading to true streaming is a future pass (backlog below). The response text is extracted from `resp.output["text"]`.
- **T16 / Chat page `model` state not threaded to IPC** — `ChatPage` holds `const [model, setModel] = useState<string>("auto")` (line 20) and wires it to `ToolDropdown` (line 96), but `sendChatMessage(priorMessages, assistantId, onChunk, onDone)` receives only the transcript and callbacks — no `model_override`. On the Rust side `chat_send_message` builds its `AiRequest` with `model_override: None` and `TaskKind::TextGeneration`, so chat currently resolves to the TextGeneration default (Claude Sonnet via strategy) regardless of the dropdown choice. Follow-up: wire the chat `model` through the TS wrapper + IPC when per-provider chat is wanted.
- **T15 / chat no longer instantiates `ClaudeCliClient` per message** — the quality-review refactor (d1b4d7a) routes chat through `AiRouterState` instead of rebuilding the client on each call. Intended side-effect: chat now honors the user's Claude transport choice (if they pinned `api`, chat uses HTTP Claude; if `cli`, chat uses the subscription CLI).
- **T19 / Graphic3D image-to-3D and Video per-segment** — only the primary text flows carry `model_override` (Text3D for Graphic3D, TextToVideo storyboard for Video). Secondary flows (Graphic3D image-to-3D, per-segment video generation) are left on auto-routing and can be threaded later if requested.

## Commit list (24 commits spanning f3ad93a → 27d0a59)

```
27d0a59 test(e2e): chat smoke + generate-with-override
b03f165 feat(shortcuts): tool-overrides section in help overlay
c6f8eba feat(modules): replicate Graphic2D ToolDropdown+Optimize+override pattern across remaining 4 pages
a644247 fix(graphic2d): parseOverride runs before optimize — slug never reaches Claude
81eac14 feat(graphic2d): wire ToolDropdown + OptimizeToggle + /tool override
d1b4d7a refactor(chat): route via AiRouter + typed errors + double-fire guard + a11y
bbcda0b feat(chat): Sidebar Chat entry above Settings + Cmd+L shortcut
5d91344 feat(chat): Chat page with streaming + Tool dropdown + Optimize
ba6cabb feat(chat): chat_send_message IPC + event-based chunk streaming
868be00 feat(chat): chatStore — Zustand + localStorage persistence
f43d961 refactor(optimize): typed RouterIpcError + aria-busy + concurrent-call guard
1ccc9ed feat(ui): OptimizeToggle — switch + busy spinner + Undo button
5e991ec feat(optimize): useOptimizePrompt hook + 4 tests
c883d27 feat(optimize): optimize_prompt IPC + 4 meta-template tests
b209461 feat(ui): ToolDropdown — capability-matrix-driven, grouped by tier
4b7ec29 feat(tooling): /tool override parser — start or end only, whitelist-checked
41bbc51 feat(tooling): toolCatalog.ts capability matrix + 4 tests
0f41113 docs(phase-9): Sub-Project A (Claude CLI Bridge) closure note
d97b03b feat(router): AiRequest.model_override — explicit primary with strategy fallbacks
adc2685 feat(settings): Claude transport selector (auto/api/cli) + CLI auto-detect display
e374264 feat(claude-cli): Tauri commands + transport-aware registry
d4bf2e0 feat(claude-cli): ClaudeCliClient with Spawner trait + 4 mock tests
49c9167 feat(claude-cli): stream-json accumulator + 5 parser tests
5cd4f4a feat(claude-cli): detect_claude_binary + candidate paths
```

Plus the design spec (f3ad93a) and implementation plan (838f1fa) that opened the phase.

## Backlog filed / future work

- True token-by-token chat streaming — swap the single-shot emit for stream-json forwarding driven by `ClaudeCliClient`
- Thread the chat ToolDropdown `model` into `sendChatMessage` + `chat_send_message` (currently pinned to the TextGeneration default, Sonnet via strategy)
- Secondary-flow overrides: Graphic3D image-to-3D, Video per-segment generation
- `ToolDropdown` a11y polish: `aria-label` on trigger + `aria-labelledby` on the listbox (pre-existing gap surfaced during T19 review)
- Prompt-injection hardening for the chat transcript — the `"assistant: ..."` role tokens are user-supplied in the flattened prompt; low priority but worth a defensive pass

## Verdict

**Phase 9 closed.** All 22 tasks landed across 24 commits. CI green on `main` at HEAD (27d0a59). Transport bridge + tool UX ready for the first live test.
