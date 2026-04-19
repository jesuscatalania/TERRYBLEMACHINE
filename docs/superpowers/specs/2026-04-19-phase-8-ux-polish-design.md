# Phase 8 — Sub-Project 2: UX-Polish (Design)

**Date:** 2026-04-19
**Phase context:** Phase 8 ("Polish & Release") decomposed into Testing → UX-Polish → Performance. Testing closed (`docs/superpowers/specs/2026-04-19-phase-8-testing-verification-report.md`). This is Sub-Project 2.
**Source plan:** `docs/ENTWICKLUNGSPLAN.md` lines 365-369 (Phase 8.2)

## Goal

Make the app **feel** finished — first-time users get a guided start, power-users get keyboard shortcuts they can discover, technical parameters carry just-in-time help, and any wait shows visible progress instead of a frozen UI.

## Discovery (existing primitives)

Already in the codebase:
- `src/components/ui/Tooltip.tsx` (used only in DesignSystem demo)
- `src/components/ui/Skeleton.tsx` (declared, unused)
- `src/components/ui/Toast.tsx` + `useUiStore.notify(...)` pattern
- `src/components/shell/Kbd.tsx` (display-only `<kbd>` styling)
- `src/hooks/useUndoRedo.ts` (Cmd+Z / Cmd+Shift+Z, suppresses in text-fields)

Most of this sub-project is **applying** the primitives broadly + adding the missing systems (onboarding modal, keyboard registry).

## Scope decisions (from brainstorming)

1. **Onboarding** = Welcome-Modal with 3 steps + localStorage skip flag
2. **Keyboard shortcuts** = Registry-based `useKeyboardShortcut` + Cmd+/ Help-Overlay
3. **Tooltips** = Icon-only sweep + HelpIcon (?) on ~5-10 technical parameters
4. **Loading states** = `LoadingButton` + Skeleton tiles + determinate progress on Toast where backend reports progress

## Non-Goals

- **Distribution** (signing/installer/auto-update) — out of Phase 8 entirely; revisited after first live-test
- **Customizable / rebindable shortcuts** — backlog
- **Cancel handles in backend pipelines** — backlog (would require AbortHandle threading per pipeline)
- **ETA estimates / queue visualization** — backlog
- **Full a11y audit (axe-core, screen-reader walkthroughs)** — backlog
- **Visual-regression tests for the new UI** — out of scope
- **Tour-with-highlight-overlays for module discovery** — Welcome-Modal text is sufficient; full tour is backlog if requested
- **Multi-window keyboard scope** — single-window app today

## Architecture — Where Things Live

| Component | Location |
|---|---|
| Welcome-Modal | `src/components/onboarding/WelcomeModal.tsx` + `WelcomeModal.test.tsx` |
| Welcome-flow hook | `src/hooks/useWelcomeFlow.ts` (reads/writes `localStorage["tm:welcome:dismissed"]`) |
| App-level mount | `src/App.tsx` mounts `<WelcomeModal />` next to `<Toaster />` |
| Keyboard registry store | `src/stores/keyboardStore.ts` (Zustand: `Map<string, ShortcutEntry>`) |
| `useKeyboardShortcut` hook | `src/hooks/useKeyboardShortcut.ts` |
| Global keyboard listener | `src/hooks/useGlobalKeyboardDispatch.ts` (mounted once in App) |
| Help-Overlay | `src/components/shell/ShortcutHelpOverlay.tsx` (lists active shortcuts grouped by scope) |
| `LoadingButton` | `src/components/ui/LoadingButton.tsx` (wraps Button + Loader2 spinner) |
| `HelpIcon` | `src/components/ui/HelpIcon.tsx` (`?` glyph + Tooltip) |
| Sidebar shortcuts | `src/components/shell/Sidebar.tsx` (registers Cmd+1-5 per module) |
| Per-page shortcuts | each `src/pages/*.tsx` registers Cmd+Enter for Generate while not in textarea |
| Existing `useUndoRedo` migration | refactor in-place to call `useKeyboardShortcut` |
| Toast progress field | `src/stores/uiStore.ts` adds optional `progress?: { current: number; total: number }` to ToastInput |
| Toast progress render | `src/components/ui/Toast.tsx` renders a thin determinate bar when `progress` present |

## Components in Detail

### 1. Welcome-Modal (Onboarding)

Single modal component with 3 steps via local `stepIndex` state. Uses existing `Modal` primitive.

**Step 1:** "Welcome to TERRYBLEMACHINE — your local AI design tool. To start generating, add API keys for the providers you want to use in **Settings**." Subtle "Open Settings" link button.

**Step 2:** "Optional: drop reference images, color palettes, and rules into the **`meingeschmack/`** folder. Every generated output is flavored by what's there. See `meingeschmack/README.md` for details."

**Step 3:** "Optional: organize generated assets per topic by creating a **project** (Cmd+N). You can ship without one, but projects keep history + exports per-topic."

**Footer:** Skip / Back / Next buttons; "Don't show this again" checkbox (default checked on the final step).

**`useWelcomeFlow()` hook:** returns `{ open, dismiss }`. Auto-opens on first mount if `localStorage["tm:welcome:dismissed"]` is not `"true"`. Calling `dismiss()` writes the flag and closes.

**App.tsx integration:** mount `<WelcomeModal />` at root next to `<Toaster />`. The hook drives `open` from the localStorage check.

### 2. Keyboard-Shortcut Registry

**Store** (`src/stores/keyboardStore.ts`):

```ts
export type ShortcutScope = "global" | "module:website" | "module:graphic2d" | ... | "page";
export interface ShortcutEntry {
  id: string;            // unique within scope+combo, used for unregister
  combo: string;         // canonical: "Mod+S", "Mod+Shift+Z", "Mod+1", "?"
  handler: () => void;
  scope: ShortcutScope;
  label: string;         // human-readable description for the help overlay
  when?: () => boolean;  // optional gating predicate (e.g., disable on /settings)
}
```

Zustand store exposes `register(entry)`, `unregister(id)`, `list()`.

**`useKeyboardShortcut`** (hook): registers on mount with a stable id, unregisters on unmount. Suppressed in `<input>`, `<textarea>`, contenteditable (mirrors `useUndoRedo`'s `isTextField`).

**Global dispatcher** (`useGlobalKeyboardDispatch`): single `keydown` listener on `document`. Canonicalizes the event into a combo string, finds matching entries (priority: page > module > global), calls the highest-priority `handler`, calls `event.preventDefault()`. Mounted once in `App.tsx`.

**Help-Overlay** (`ShortcutHelpOverlay`): Modal showing `useKeyboardStore.list()` grouped by scope. Triggered by `Cmd+/` or `?` (registered as a global shortcut by App.tsx).

**Migration of `useUndoRedo`:** internal body switches to `useKeyboardShortcut({ combo: "Mod+Z", scope: "global", label: "Undo", handler: () => store.undo() })` + `Mod+Shift+Z` for redo. Public hook signature unchanged; existing tests pass without modification.

**Initial shortcut set:**
- Global: `Mod+1` … `Mod+5` (modules), `Mod+/` (help), `Mod+Z` / `Mod+Shift+Z` (undo/redo via migration), `Mod+N` (new project)
- Per-page: `Mod+Enter` (Generate) on each module's prompt textarea while focus is in it (page-scoped — registered in each `*.tsx`)

### 3. Tooltips

**Icon-only sweep (no new component, just wrap):**
- `LogoGallery.tsx`: heart button → Tooltip "Toggle favorite"
- `Sidebar.tsx`: each module link → Tooltip with module name + Cmd+N hint
- Other icon-only buttons surface during impl by grep — wrap as found

**Help-Icon for technical parameters:**
- `src/components/ui/HelpIcon.tsx` — renders 12px circled `?` glyph; hover triggers Tooltip with the explanation text
- Applied next to: `filter_speckle` slider (Vectorizer), `corner_threshold` slider, `color_mode` toggle, `kerning` slider (TextLogoControls), `cfg_scale` + `steps` + `seed` (image generation pages)

Each HelpIcon's content is a single sentence: "Drops clusters smaller than NxN pixels — higher = cleaner trace, lower = preserves detail."

### 4. Loading-States

**`LoadingButton`** (`src/components/ui/LoadingButton.tsx`): wraps existing Button. Props extend ButtonProps with `loading?: boolean`. When loading: button is disabled, label is replaced by `<Loader2 className="animate-spin" />` + label text in muted color. Replaces every `busy ? "Generating…" : "Generate"` pattern across modules.

**Skeleton tiles in galleries:**
- `LogoGallery.tsx`: when `busy && variants.length === 0`, render 6 `<Skeleton />` tiles in the same 3-column grid
- `Graphic2D.tsx` variant grid: same pattern
- Other gallery components surface during impl

**Determinate Progress on Toast:**
- `useUiStore.ToastInput` gains optional `progress?: { current: number; total: number }`
- `Toast.tsx` renders a thin 2px bar at toast-bottom showing `current/total` ratio when `progress` present
- Backend pipelines that already emit progress events (Remotion render, Brand-kit export) wire `progress` into their toasts; pipelines without progress events render no bar (no-op)
- **No backend changes required** — the existing `notify` already accepts arbitrary props through TypeScript widening; we just narrow the type

## Data Flow

**Onboarding:** localStorage flag → `useWelcomeFlow` → modal open state → user dismisses → flag set.

**Shortcuts:** Components call `useKeyboardShortcut(opts)` → entry registered in store → global dispatcher matches keydown events → handler fires.

**Tooltips/HelpIcons:** Pure render — no state.

**LoadingButton:** Pass-through of existing busy state.

**Toast progress:** Caller passes `progress` field → Toast renders bar.

## Testing

- `WelcomeModal.test.tsx` — opens on no flag, hides on flag set, 3 steps navigate, "don't show again" sets flag
- `useWelcomeFlow.test.ts` — flag round-trip
- `keyboardStore.test.ts` — register/unregister, list grouping
- `useKeyboardShortcut.test.tsx` — text-field suppression, scope priority, lifecycle
- `useUndoRedo.test.tsx` — existing tests must still pass after migration (regression gate)
- `ShortcutHelpOverlay.test.tsx` — Cmd+/ opens, Esc closes, lists active shortcuts grouped
- `LoadingButton.test.tsx` — loading=true → disabled + spinner; click handler suppressed
- `HelpIcon.test.tsx` — Tooltip renders on hover, content matches prop
- `Toast.test.tsx` — progress bar shown when present, hidden otherwise
- LogoGallery.test.tsx extension — skeleton tiles when busy + empty
- E2E: extend `e2e/tests/navigation.spec.ts` with Cmd+1-5 module switching; new `e2e/tests/welcome.spec.ts` asserts modal appears on fresh localStorage + dismisses

## Risks

| Risk | Mitigation |
|---|---|
| Cmd+1-5 collides with browser tab-switching in dev mode | `event.preventDefault()` in dispatcher |
| Skeleton-tile count divergence (6 fixed but variants count is dynamic) | Pass expected count as prop; fallback default 6 |
| `useKeyboardShortcut` re-registers on every render if handler isn't memoized | Hook accepts handler as prop; memoize with useEvent / useCallback in callers; document in JSDoc |
| Welcome modal blocks tests | localStorage flag set in `src/test/setup.ts` so tests boot dismissed by default |
| Migrating `useUndoRedo` breaks existing test | Gate: existing test file MUST pass unchanged after migration |

## Acceptance Criteria

This sub-project lands when:
- `<WelcomeModal />` renders on first launch (no localStorage flag), 3 steps navigate, dismiss persists
- `useKeyboardShortcut` registry works: Cmd+1-5 switches modules, Cmd+/ opens help overlay listing all active shortcuts, Cmd+Enter on prompt fields triggers Generate
- `useUndoRedo` migrated to registry without breaking its existing test
- Icon-only buttons across the app have Tooltips
- HelpIcons present next to ≥5 technical-parameter controls (filter_speckle, corner_threshold, kerning, cfg_scale, steps + others as found)
- All "Generating…" / "Vectorizing…" patterns replaced by `<LoadingButton loading={...}>`
- Skeleton tiles render in LogoGallery and Graphic2D variant grids when busy + empty
- Toast supports optional progress prop; ≥1 caller actually emits progress
- All existing tests pass; new tests added per Testing section
- E2E specs (navigation + welcome) pass in CI
- `pnpm biome check .` clean, CI all 5 jobs green

After landing, brainstorm Sub-Project 3 (Performance).
