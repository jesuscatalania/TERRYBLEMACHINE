# Phase 9: Claude Subscription Bridge + Tool-Selection UX (Design)

**Date:** 2026-04-19
**Phase context:** Phase 8 closed (Testing / UX-Polish / Performance + Hardening waves). Live-test pending. Phase 9 ships two tightly-coupled feature blocks before live-test:
1. Replace HTTP Anthropic API with the local Claude CLI subscription bridge for ALL backend Claude calls
2. Sidebar Chat entry + per-prompt tool-selection UX (dropdown + `/tool` override + Optimize ON/OFF)
**Source request:** User decided architecture directly (no brainstorming Q&A). Architecture decisions captured below.

## Goal

Two simultaneous wins:
1. **Zero per-token cost for backend Claude usage** by routing through the user's existing Claude Pro/Max subscription via the official `claude` CLI sidecar. All 5 backend Claude callers (storyboard / code-gen / assist / taste-engine / image-pipeline-prompt-rewrite) use this transport.
2. **Fine-grained tool control** in every Generate flow: dropdown shows all capable models per TaskKind ordered by priority, `/tool` override syntax for power-users, Optimize toggle that runs the prompt through Claude before dispatch with full undo.

## Non-Goals

- Replacing the entire module-tab UI with a chat-only layout (Variant X from earlier discussion). Chat is **additive** — sidebar entry above Settings, opens its own page; the 5 module tabs stay unchanged.
- Customizable / rebindable shortcuts (still backlog).
- Tool-Use orchestration where Claude in chat autonomously calls module IPC commands (deferred to a Phase 10 if user wants it).
- Web-Browser bridge to claude.ai (we use the CLI; no Playwright DOM scraping).
- Mobile / Windows ports (macOS-only target stays).

## Architecture Decisions (locked in)

### Sub-Project A — Claude CLI Bridge

| Decision | Choice | Rationale |
|---|---|---|
| Transport | `claude -p "<prompt>" --output-format stream-json` spawned via `tokio::process::Command` | Official Anthropic CLI; supports streaming JSON; works with subscription |
| Auth | User runs `claude login` once (handled by CLI itself) | No new auth code in our app |
| Discovery | `which claude` at startup; fallback to `~/.claude/local/claude`; if neither found, surface a Setup-Hint in Settings | Keeps the binary user-managed; no bundling |
| Trait fit | New struct `ClaudeCliClient` implements existing `AiClient` trait (drop-in alternative to `ClaudeClient`) | No router or pipeline code changes |
| Selection | `Provider::Claude` slot in registry can hold EITHER ClaudeClient (HTTP) OR ClaudeCliClient (Subscription) based on Settings flag | Single source of truth in keychain-state; provider abstraction unchanged |
| Settings UI | Claude row in Settings-Modal gets a tri-state: `Auto / API Key / Subscription via CLI`. Auto = prefer CLI if `which claude` succeeds, else API Key | Discoverability + sane default |
| Backwards-compat | If user has neither CLI installed nor API key, all 5 backend callers fail-fast with a clear error toast | Honest error path, no silent degradation |
| Model selection | CLI's `--model claude-opus-4-7` flag maps from our existing `Model::ClaudeOpus/Sonnet/Haiku` | One-line conversion |
| Tool-Use | Sub-Project A does NOT use Claude Code's built-in tool-use (Read/Write/Bash/etc.). We call it as a pure text generator via `-p` | Tool-Use orchestration is Phase 10 |
| Streaming | Parse `stream-json` line-by-line, accumulate text content | Matches Claude HTTP streaming semantics |
| Error mapping | CLI exit-codes → ProviderError variants (1=Permanent, 130=Cancelled-as-Permanent, network/auth from JSON error blocks) | Existing trait contract |

### Sub-Project B — Chat Panel + Tool UX

| Decision | Choice | Rationale |
|---|---|---|
| Chat layout | Sidebar entry "Chat" placed ABOVE the Settings gear; opens a full-page Chat module (not a modal) | User explicitly: "links über Settings" |
| Chat backend | Drives ClaudeCliClient (from Sub-Project A) directly via a new IPC `chat_send_message` | Keeps it simple, no router round-trip for plain conversation |
| Chat history | Persistent per-session in localStorage; cleared on "New chat" button | No project-level persistence yet (backlog if wanted) |
| Tool override syntax | `/tool` at START or END of prompt only (never mid-string). Slug format: `<provider>-<model>-<version>` (e.g. `/fal-kling-v15`, `/claude-opus`) plus short aliases (`/kling` → V2-Master, `/sdxl`, `/runway`, etc.) | User explicitly specified; `/` is unlikely to appear in normal prose at start/end |
| Override visibility | Toast + chip in the prompt-input show "Using @<model> override" when detected; clears as user edits | Discoverability without being noisy |
| Tool dropdown | Per prompt-field, shows ALL capable models for the TaskKind ordered: Primary → Fallbacks → Alternatives → "Auto (default)" | User explicitly: discoverable + clickable |
| Dropdown source | Capability matrix lives in `src/lib/toolCatalog.ts` mapping TaskKind → `ToolDef[]` with `tier: "primary" | "fallback" | "alternative"` | Single source of truth, easy to extend |
| Optimize button | ON/OFF toggle next to prompt input. When ON + Generate clicked: (1) prompt → ClaudeCliClient with optimization meta-prompt → returns improved prompt; (2) **Frontend replaces input value with optimized prompt + records original** ; (3) Undo button appears for 30s (or until next generate); (4) Generate then proceeds with the new prompt | User explicitly: replace + undo |
| Optimize meta-prompt | "You are an expert prompt engineer for AI image/video/code generation. Rewrite the user's prompt to be more specific, visually rich, and unambiguous, while preserving their intent. Output ONLY the rewritten prompt — no preamble, no explanation." (parameterized by TaskKind for tone) | Consistent quality bar |
| Shortcuts modal | New entry in existing ShortcutHelpOverlay (`Cmd+/` or `?`): adds a "Tool Overrides" section listing every `/tool` slug with its expanded model name | Reuses existing modal |
| Override priority | When BOTH dropdown AND `/tool` are set → `/tool` wins (explicit text always trumps UI state) + a console warning | Predictable, power-user-friendly |
| Override + Optimize | Order is: (1) parse override + strip from prompt → (2) Optimize the cleaned prompt if toggle ON → (3) prepend override slug back to optimized prompt → (4) dispatch | Optimizer doesn't see the slug; user sees their override survived |

## Architecture — Where Things Live

| Component | Location |
|---|---|
| **Sub-Project A** | |
| ClaudeCliClient | `src-tauri/src/api_clients/claude_cli.rs` (new) — implements `AiClient` |
| CLI discovery | `src-tauri/src/api_clients/claude_cli.rs::detect_claude_binary` — `which claude`, fallbacks, `Option<PathBuf>` cached |
| Settings Claude transport | `src/components/settings/providers.ts` ProviderDef gains `transports?: ("api" | "cli")[]`. Claude row gets a small select |
| Selection persistence | localStorage key `tm:claude:transport` = `auto | api | cli` |
| Registry switch | `src-tauri/src/api_clients/registry.rs::build_default_clients` reads keystore meta key `__claude_transport__` (or env var) and constructs ClaudeCliClient OR ClaudeClient accordingly |
| **Sub-Project B** | |
| Chat page | `src/pages/Chat.tsx` + `src/pages/Chat.test.tsx` |
| Chat IPC | `src-tauri/src/api_clients/claude_cli.rs::chat_send_message` Tauri command (streams via Tauri events) |
| Chat history store | `src/stores/chatStore.ts` (Zustand + localStorage hydration) |
| Chat sidebar entry | `src/components/shell/Sidebar.tsx` adds "Chat" link above the Settings gear |
| Chat route | `src/App.tsx` `/chat` route lazy-loaded |
| Tool catalog | `src/lib/toolCatalog.ts` + tests — TaskKind → ToolDef[] |
| Override parser | `src/lib/promptOverride.ts` + tests — `parseOverride(prompt) → { override?: string, cleanPrompt: string }` |
| Override TS types | `src/lib/promptOverride.ts` exports `ToolOverride` + alias map |
| Tool dropdown | `src/components/ui/ToolDropdown.tsx` + tests — used in every page's brief-row + Chat |
| Optimize button + Undo | `src/components/ui/OptimizeToggle.tsx` + `src/hooks/useOptimizePrompt.ts` |
| Optimize IPC | new Tauri command `optimize_prompt(text, task_kind) → string` (calls ClaudeCliClient internally) |
| Shortcuts/Overrides modal | extend existing `src/components/shell/ShortcutHelpOverlay.tsx` with second section |
| Per-page integration | each `src/pages/*.tsx` adopts `<ToolDropdown taskKind={...}>` + `<OptimizeToggle>` + override-aware `handleGenerate` |

## Capability Matrix (Sub-Project B foundation)

The dropdown per TaskKind shows these tiers:

| TaskKind | Primary | Fallbacks | Alternatives (shown but not in default chain) |
|---|---|---|---|
| TextGeneration | Claude Opus | Claude Sonnet, Claude Haiku | — |
| ImageGeneration (Simple) | fal SDXL | fal Flux Pro, Replicate Flux Dev | — |
| ImageGeneration (Medium/Complex) | fal Flux Pro | Replicate Flux Dev | fal SDXL |
| ImageEdit | fal Flux Pro | Replicate Flux Dev | — |
| Inpaint | fal Flux Fill | — | — |
| Upscale | fal Real-ESRGAN | — | — |
| Logo | Ideogram v3 | — | — |
| TextToVideo / ImageToVideo | fal Kling V2 Master | fal Kling V1.5, Runway Gen-3, Higgsfield | — |
| VideoMontage | Shotstack | — | Remotion (lokal) |
| Text3D | Meshy Text-3D | — | TripoSR (Replicate) |
| Image3D (Simple) | TripoSR | Meshy Image-3D | — |
| Image3D (Medium/Complex) | Meshy Image-3D | TripoSR | — |
| ImageAnalysis | Claude Sonnet | Claude Haiku | — |
| DepthMap | Replicate Depth-Anything v2 | — | — |

## Override Slug Aliases (Sub-Project B)

```
/claude              → Claude Sonnet (sensible default)
/claude-opus         → Claude Opus
/claude-sonnet       → Claude Sonnet
/claude-haiku        → Claude Haiku
/sdxl                → fal SDXL
/flux                → fal Flux Pro
/flux-dev            → Replicate Flux Dev
/flux-fill           → fal Flux Fill (inpaint)
/upscale             → fal Real-ESRGAN
/ideogram            → Ideogram V3 (logos)
/kling               → fal Kling V2 Master (default video)
/kling-v15           → fal Kling V1.5
/runway              → Runway Gen-3
/higgsfield          → Higgsfield
/shotstack           → Shotstack (video montage cloud)
/remotion            → Remotion (video montage local)
/meshy               → Meshy (3D, text or image based on context)
/triposr             → Replicate TripoSR (3D)
/depth               → Replicate Depth-Anything v2
```

Unknown slug → ignore + toast "Unknown override `/xyz` — falling back to default routing".

## Data Flow

### Chat message
```
User types in Chat page → Generate clicked
  → frontend invokes `chat_send_message(text, history)`
  → backend: ClaudeCliClient.chat() spawns `claude -p ... --output-format stream-json`
  → backend streams parsed text content via tauri::Event "chat:stream-chunk"
  → frontend appends chunks to current assistant message in chatStore
  → CLI exits → backend emits "chat:stream-done"
  → frontend marks message complete + persists to localStorage
```

### Generate from any module (with all new features)
```
User in Graphic2D types "ein sonnenuntergang /flux"
  → click Generate
  → handleGenerate:
    1. parseOverride("ein sonnenuntergang /flux") → { override: "flux", cleanPrompt: "ein sonnenuntergang" }
    2. If Optimize ON:
       a. await optimize_prompt(cleanPrompt, task) → "warm terracotta sunset over berlin rooftops, golden hour, cinematic 35mm film, soft grain"
       b. setPromptInputValue("warm terracotta sunset over berlin rooftops... /flux")  ← UI input is replaced
       c. record original "ein sonnenuntergang /flux" in undoStack
       d. show Undo button for 30s
       e. cleanPrompt = "warm terracotta sunset over berlin rooftops..."
    3. Resolve override: "flux" → Model::FalFluxPro; build AiRequest { task, model_override: Some(FalFluxPro), prompt: cleanPrompt + reattached override slug if any }
    4. Dispatch via existing image_pipeline → router (router respects model_override)
    5. Result rendered as today
```

### Optimize Undo
```
User clicks Undo → setPromptInputValue(undoStack.pop()) → button hides → next generate uses original
```

## Error Handling

- CLI not found → Settings shows red banner "Install Claude CLI: `brew install anthropic/claude-code/claude`" + falls back to API key path silently if user has API key
- CLI not authenticated → first call returns auth error parsed from stderr → toast "Run `claude login` in terminal first"
- Optimize fails → toast "Optimize failed, using your original prompt" + Generate proceeds with original (no replacement happened)
- Override slug unknown → toast warn + default routing
- Override + dropdown conflict → override wins, console.warn

## Testing

| Area | Tests |
|---|---|
| ClaudeCliClient | mock spawned process via TestableSpawner trait → returns canned stream-json → assert AiResponse parse + error mapping (Permanent on exit 1, Auth on JSON auth error block, Timeout on slow exit) |
| CLI discovery | mock `which` results → assert detect returns Some/None correctly |
| Chat IPC | unit-test the chat_send_message handler with mocked ClaudeCliClient |
| Chat store | history persistence, new-chat clears, chunked-message accumulation |
| Override parser | 12+ unit tests covering start/end/middle/no-override/unknown-slug/multiple-overrides (only first wins)/case-insensitivity |
| Tool catalog | one assertion per TaskKind that the catalog matches the routing strategy primary |
| ToolDropdown | renders all tiers, visually grouped, selecting an option fires onChange with Model |
| OptimizeToggle | when ON: clicking Generate calls optimize_prompt then setPromptValue; Undo restores |
| Per-page: Graphic2D | full happy-path with override + optimize toggle (mocked optimize_prompt) |
| E2E settings.spec.ts | Claude transport tri-state visible + click-able |
| E2E chat.spec.ts | new — type, send, receive (mocked) |
| E2E generate-with-override.spec.ts | type "/flux prompt", click generate, assert mocked invoke received Model::FalFluxPro |

## Acceptance Criteria

- `claude` CLI auto-detected on launch; status visible in Settings
- All 5 backend Claude callers route through ClaudeCliClient when transport=cli
- Chat page accessible from sidebar entry above Settings; messages stream; persists across reload
- Every page's prompt-input has a Tool dropdown showing the catalog for that TaskKind
- `/tool` syntax at start/end works; mid-string ignored
- Optimize toggle replaces prompt with optimized version; Undo restores
- ShortcutHelpOverlay shows all `/tool` aliases
- Existing flows (existing tests) all still pass
- All 5 CI jobs green

## Risks

| Risk | Mitigation |
|---|---|
| User has no `claude` CLI installed → backend Claude calls fail | Fall back to API-key transport if available; otherwise clear setup banner |
| `claude -p` output format changes | Parse defensively; surface unparseable lines as raw text in Chat |
| Optimize prompt overruns Claude context | Limit input to 8KB + meta-prompt; Claude returns text-only |
| Replace-prompt-on-Optimize might surprise user | Explicit Undo button + 30s window; toast announces "Prompt optimized — Undo available" |
| `/tool` collisions with prompt content (rare prose starting with `/`) | Strict slug whitelist — only known aliases trigger; unknown `/xyz` is ignored verbatim |
| Tool dropdown bloats every page | Inline as small icon-button next to prompt; expands on click |
| Sidecar process leaks if Tauri force-quits during streaming | Spawn with `kill_on_drop(true)` + Tauri shutdown handler aborts pending children |
| Claude CLI prompts for confirmation interactively | `--print` (`-p`) flag is non-interactive; verify version compatibility |
