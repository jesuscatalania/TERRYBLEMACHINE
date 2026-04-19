# Phase 8 Sub-Project 2: UX-Polish — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a 3-step Welcome-Modal onboarding, a registry-based keyboard-shortcut system with help overlay, broad Tooltip + HelpIcon coverage, and unified loading affordances (LoadingButton + Skeleton tiles + determinate Toast progress).

**Architecture:** Four parallel pillars. Onboarding adds a localStorage-gated modal at the App root. Keyboard adds a Zustand registry + a single global keydown listener that dispatches by scope priority; existing `useUndoRedo` migrates onto it without API change. Tooltips reuse the existing `Tooltip` primitive — the work is wrapping icon-only triggers and adding a small `HelpIcon` component for technical-parameter help. Loading adds `LoadingButton` (Button + Loader2 spinner), Skeleton tiles in gallery components when busy+empty, and an optional `progress: { current, total }` field on Toast.

**Tech Stack:** React 19 + TypeScript strict + Tailwind + Zustand + Vitest + framer-motion (already in tree). Lucide-react `Loader2` icon. No new deps.

**Source spec:** `docs/superpowers/specs/2026-04-19-phase-8-ux-polish-design.md`

---

## File Structure

| File | Responsibility |
|---|---|
| `src/hooks/useWelcomeFlow.ts` | localStorage flag + open/dismiss state |
| `src/hooks/useWelcomeFlow.test.ts` | Flag round-trip |
| `src/components/onboarding/WelcomeModal.tsx` | 3-step modal driving `useWelcomeFlow` |
| `src/components/onboarding/WelcomeModal.test.tsx` | Step navigation, dismiss, "don't show again" |
| `src/stores/keyboardStore.ts` | Zustand registry: register/unregister/list |
| `src/stores/keyboardStore.test.ts` | Lifecycle + grouping |
| `src/hooks/useKeyboardShortcut.ts` | Hook that registers an entry, unregisters on unmount |
| `src/hooks/useKeyboardShortcut.test.tsx` | Mount/unmount, scope priority, text-field suppression |
| `src/hooks/useGlobalKeyboardDispatch.ts` | Single document keydown listener; canonicalizes combo; dispatches highest-priority handler |
| `src/components/shell/ShortcutHelpOverlay.tsx` | Modal listing active shortcuts grouped by scope |
| `src/components/shell/ShortcutHelpOverlay.test.tsx` | Cmd+/ opens, Esc closes, lists shortcuts |
| `src/hooks/useUndoRedo.ts` | Refactored in-place to call `useKeyboardShortcut` (public signature unchanged) |
| `src/components/ui/LoadingButton.tsx` | Button wrapper with `loading?: boolean` prop |
| `src/components/ui/LoadingButton.test.tsx` | Spinner + disabled when loading |
| `src/components/ui/HelpIcon.tsx` | `?` glyph + Tooltip |
| `src/components/ui/HelpIcon.test.tsx` | Tooltip renders on hover |
| `src/stores/uiStore.ts` | Add optional `progress?: { current: number; total: number }` to Notification + NotificationInput |
| `src/components/ui/Toast.tsx` | Render thin determinate bar when `progress` present |
| `src/test/setup.ts` | Pre-set `localStorage["tm:welcome:dismissed"] = "true"` so unit tests don't see the modal |
| `src/App.tsx` | Mount `<WelcomeModal />` + `<ShortcutHelpOverlay />` + `useGlobalKeyboardDispatch()` |
| `src/components/shell/Sidebar.tsx` | Wrap module links in Tooltip; register Cmd+1-5 |
| `src/components/typography/LogoGallery.tsx` | Wrap heart button in Tooltip; render Skeleton tiles when busy+empty |
| `src/pages/Typography.tsx` | Replace `busy ? "Generating…" : "Generate"` with `<LoadingButton loading={busy}>`; HelpIcon on kerning slider; Cmd+Enter on prompt |
| `src/pages/WebsiteBuilder.tsx`, `Graphic2D.tsx`, `Graphic3D.tsx`, `Video.tsx` | Same: LoadingButton swap; Cmd+Enter wiring; HelpIcons on technical params |
| `src/components/typography/TextLogoControls.tsx` | HelpIcon next to Kerning label |
| `src/components/typography/SvgEditor.tsx` (only if vectorize controls live there) | HelpIcon next to filter_speckle / corner_threshold / color_mode |
| `e2e/tests/welcome.spec.ts` | First-load shows modal, dismiss persists |
| `e2e/tests/navigation.spec.ts` | Extend with Cmd+1-5 module-switch assertion |
| `docs/superpowers/specs/2026-04-19-phase-8-ux-polish-verification-report.md` | Sub-Project closure |

---

## Task 1: useWelcomeFlow hook + localStorage gate

**Files:**
- Create: `src/hooks/useWelcomeFlow.ts`
- Create: `src/hooks/useWelcomeFlow.test.ts`
- Modify: `src/test/setup.ts` (add localStorage pre-set so unit tests boot dismissed)

- [ ] **Step 1: Write failing test**

```ts
// src/hooks/useWelcomeFlow.test.ts
import { renderHook, act } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { useWelcomeFlow, WELCOME_LOCALSTORAGE_KEY } from "@/hooks/useWelcomeFlow";

describe("useWelcomeFlow", () => {
  afterEach(() => {
    window.localStorage.removeItem(WELCOME_LOCALSTORAGE_KEY);
  });

  it("opens by default when no flag is set", () => {
    window.localStorage.removeItem(WELCOME_LOCALSTORAGE_KEY);
    const { result } = renderHook(() => useWelcomeFlow());
    expect(result.current.open).toBe(true);
  });

  it("stays closed when flag is set", () => {
    window.localStorage.setItem(WELCOME_LOCALSTORAGE_KEY, "true");
    const { result } = renderHook(() => useWelcomeFlow());
    expect(result.current.open).toBe(false);
  });

  it("dismiss() sets flag and closes", () => {
    window.localStorage.removeItem(WELCOME_LOCALSTORAGE_KEY);
    const { result } = renderHook(() => useWelcomeFlow());
    act(() => result.current.dismiss());
    expect(result.current.open).toBe(false);
    expect(window.localStorage.getItem(WELCOME_LOCALSTORAGE_KEY)).toBe("true");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm vitest run src/hooks/useWelcomeFlow
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement the hook**

```ts
// src/hooks/useWelcomeFlow.ts
import { useCallback, useState } from "react";

export const WELCOME_LOCALSTORAGE_KEY = "tm:welcome:dismissed";

export interface WelcomeFlowApi {
  open: boolean;
  dismiss: () => void;
}

function isDismissed(): boolean {
  if (typeof window === "undefined") return true; // SSR-safe
  return window.localStorage.getItem(WELCOME_LOCALSTORAGE_KEY) === "true";
}

/**
 * Drives the first-launch onboarding modal. The modal opens automatically
 * unless the user has previously dismissed it (`localStorage` flag).
 */
export function useWelcomeFlow(): WelcomeFlowApi {
  const [open, setOpen] = useState<boolean>(() => !isDismissed());

  const dismiss = useCallback(() => {
    if (typeof window !== "undefined") {
      window.localStorage.setItem(WELCOME_LOCALSTORAGE_KEY, "true");
    }
    setOpen(false);
  }, []);

  return { open, dismiss };
}
```

- [ ] **Step 4: Pre-dismiss in test setup so other tests don't see the modal**

Append to `src/test/setup.ts`:

```ts
// Pre-dismiss the Welcome modal in unit tests so it doesn't sit on top of
// every render. E2E tests clear this flag explicitly when they want to
// exercise the onboarding flow.
import { WELCOME_LOCALSTORAGE_KEY } from "@/hooks/useWelcomeFlow";
beforeEach(() => {
  window.localStorage.setItem(WELCOME_LOCALSTORAGE_KEY, "true");
});
```

- [ ] **Step 5: Run test, verify pass**

```bash
pnpm vitest run src/hooks/useWelcomeFlow
```

Expected: 3 PASS.

- [ ] **Step 6: Commit**

```bash
git add src/hooks/useWelcomeFlow.ts src/hooks/useWelcomeFlow.test.ts src/test/setup.ts
git commit -m "feat(onboarding): useWelcomeFlow hook + test-setup localStorage pre-dismiss"
```

---

## Task 2: WelcomeModal — 3 steps + dismiss

**Files:**
- Create: `src/components/onboarding/WelcomeModal.tsx`
- Create: `src/components/onboarding/WelcomeModal.test.tsx`

- [ ] **Step 1: Write failing test**

```tsx
// src/components/onboarding/WelcomeModal.test.tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { WelcomeModal } from "@/components/onboarding/WelcomeModal";
import { WELCOME_LOCALSTORAGE_KEY } from "@/hooks/useWelcomeFlow";

describe("WelcomeModal", () => {
  afterEach(() => {
    window.localStorage.removeItem(WELCOME_LOCALSTORAGE_KEY);
  });

  it("does not render when localStorage flag is set", () => {
    window.localStorage.setItem(WELCOME_LOCALSTORAGE_KEY, "true");
    render(<WelcomeModal />);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("renders step 1 by default when flag missing", () => {
    window.localStorage.removeItem(WELCOME_LOCALSTORAGE_KEY);
    render(<WelcomeModal />);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText(/Welcome to TERRYBLEMACHINE/i)).toBeInTheDocument();
  });

  it("Next advances through steps; Back returns; Skip + Done dismiss", async () => {
    const user = userEvent.setup();
    window.localStorage.removeItem(WELCOME_LOCALSTORAGE_KEY);
    render(<WelcomeModal />);
    await user.click(screen.getByRole("button", { name: /Next/i }));
    expect(screen.getByText(/meingeschmack/i)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Next/i }));
    expect(screen.getByText(/create a project/i)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Back/i }));
    expect(screen.getByText(/meingeschmack/i)).toBeInTheDocument();

    // Done on the last step dismisses
    await user.click(screen.getByRole("button", { name: /Next/i })); // back to step 3
    await user.click(screen.getByRole("button", { name: /Done/i }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(window.localStorage.getItem(WELCOME_LOCALSTORAGE_KEY)).toBe("true");
  });

  it("Skip dismisses without completing", async () => {
    const user = userEvent.setup();
    window.localStorage.removeItem(WELCOME_LOCALSTORAGE_KEY);
    render(<WelcomeModal />);
    await user.click(screen.getByRole("button", { name: /Skip/i }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(window.localStorage.getItem(WELCOME_LOCALSTORAGE_KEY)).toBe("true");
  });
});
```

- [ ] **Step 2: Run, verify fail**

```bash
pnpm vitest run src/components/onboarding/
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

```tsx
// src/components/onboarding/WelcomeModal.tsx
import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Modal } from "@/components/ui/Modal";
import { useWelcomeFlow } from "@/hooks/useWelcomeFlow";

const STEPS = [
  {
    title: "Welcome to TERRYBLEMACHINE",
    body: "Your local AI design tool. To start generating, add API keys for the providers you want to use in Settings.",
  },
  {
    title: "Flavor every output via meingeschmack/",
    body: "Optional: drop reference images, color palettes, and rules into the meingeschmack/ folder. Every generated output is flavored by what's there. See meingeschmack/README.md for details.",
  },
  {
    title: "Organize with projects",
    body: "Optional: create a project (Cmd+N) to organize generated assets per topic. You can ship without one, but projects keep history + exports per-topic.",
  },
] as const;

export function WelcomeModal() {
  const { open, dismiss } = useWelcomeFlow();
  const [step, setStep] = useState(0);
  if (!open) return null;
  const isLast = step === STEPS.length - 1;
  const current = STEPS[step];

  return (
    <Modal
      open={open}
      onClose={dismiss}
      title={current.title}
      maxWidth={480}
      footer={
        <>
          <Button variant="ghost" size="sm" onClick={dismiss}>
            Skip
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setStep((s) => Math.max(0, s - 1))}
            disabled={step === 0}
          >
            Back
          </Button>
          {isLast ? (
            <Button variant="primary" size="sm" onClick={dismiss}>
              Done
            </Button>
          ) : (
            <Button variant="primary" size="sm" onClick={() => setStep((s) => s + 1)}>
              Next
            </Button>
          )}
        </>
      }
    >
      <div className="flex flex-col gap-3 text-2xs text-neutral-dark-200 leading-relaxed">
        <p>{current.body}</p>
        <p className="font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
          Step {step + 1} of {STEPS.length}
        </p>
      </div>
    </Modal>
  );
}
```

- [ ] **Step 4: Mount in App.tsx**

In `src/App.tsx`, import and add `<WelcomeModal />` near the existing `<Toaster />`:

```tsx
import { WelcomeModal } from "@/components/onboarding/WelcomeModal";
// ...inside the App return, near <Toaster />:
<WelcomeModal />
```

- [ ] **Step 5: Run tests**

```bash
pnpm vitest run src/components/onboarding/ src/App
```

Expected: PASS for new tests; existing App tests still pass (Welcome modal is suppressed by `setup.ts`'s pre-dismiss).

- [ ] **Step 6: Biome + commit**

```bash
pnpm biome check .
git add src/components/onboarding/ src/App.tsx
git commit -m "feat(onboarding): 3-step Welcome modal mounted in App root"
```

---

## Task 3: Keyboard registry store

**Files:**
- Create: `src/stores/keyboardStore.ts`
- Create: `src/stores/keyboardStore.test.ts`

- [ ] **Step 1: Write failing test**

```ts
// src/stores/keyboardStore.test.ts
import { afterEach, describe, expect, it } from "vitest";
import { useKeyboardStore } from "@/stores/keyboardStore";

describe("keyboardStore", () => {
  afterEach(() => {
    useKeyboardStore.setState({ entries: new Map() });
  });

  it("register adds entry; list returns it", () => {
    const handler = () => {};
    useKeyboardStore.getState().register({
      id: "test:undo",
      combo: "Mod+Z",
      handler,
      scope: "global",
      label: "Undo",
    });
    expect(useKeyboardStore.getState().list()).toHaveLength(1);
    expect(useKeyboardStore.getState().list()[0]?.label).toBe("Undo");
  });

  it("unregister removes entry by id", () => {
    useKeyboardStore.getState().register({
      id: "x",
      combo: "Mod+S",
      handler: () => {},
      scope: "global",
      label: "Save",
    });
    useKeyboardStore.getState().unregister("x");
    expect(useKeyboardStore.getState().list()).toHaveLength(0);
  });

  it("multiple entries with same combo coexist (priority resolved by dispatcher, not store)", () => {
    useKeyboardStore.getState().register({
      id: "g",
      combo: "Mod+Enter",
      handler: () => {},
      scope: "global",
      label: "global",
    });
    useKeyboardStore.getState().register({
      id: "p",
      combo: "Mod+Enter",
      handler: () => {},
      scope: "page",
      label: "page",
    });
    expect(useKeyboardStore.getState().list()).toHaveLength(2);
  });

  it("entriesByCombo groups by canonical combo", () => {
    useKeyboardStore.getState().register({
      id: "a",
      combo: "Mod+Enter",
      handler: () => {},
      scope: "global",
      label: "a",
    });
    useKeyboardStore.getState().register({
      id: "b",
      combo: "Mod+Enter",
      handler: () => {},
      scope: "page",
      label: "b",
    });
    const grouped = useKeyboardStore.getState().entriesByCombo("Mod+Enter");
    expect(grouped).toHaveLength(2);
  });
});
```

- [ ] **Step 2: Run, fail**

```bash
pnpm vitest run src/stores/keyboardStore
```

- [ ] **Step 3: Implement**

```ts
// src/stores/keyboardStore.ts
import { create } from "zustand";

export type ShortcutScope =
  | "global"
  | "page"
  | "module:website"
  | "module:graphic2d"
  | "module:graphic3d"
  | "module:video"
  | "module:typography";

export interface ShortcutEntry {
  /** Stable id within (scope, combo) — used for unregister. */
  id: string;
  /** Canonical combo string, e.g. "Mod+Z", "Mod+Shift+Z", "Mod+1", "?". */
  combo: string;
  handler: () => void;
  scope: ShortcutScope;
  /** Human-readable description for the help overlay. */
  label: string;
  /** Optional gating predicate (e.g., disable when on /settings). */
  when?: () => boolean;
}

interface KeyboardState {
  entries: Map<string, ShortcutEntry>;
  register: (entry: ShortcutEntry) => void;
  unregister: (id: string) => void;
  list: () => ShortcutEntry[];
  /** All entries matching a combo, in registration order. */
  entriesByCombo: (combo: string) => ShortcutEntry[];
}

export const useKeyboardStore = create<KeyboardState>((set, get) => ({
  entries: new Map(),
  register: (entry) =>
    set((state) => {
      const next = new Map(state.entries);
      next.set(entry.id, entry);
      return { entries: next };
    }),
  unregister: (id) =>
    set((state) => {
      const next = new Map(state.entries);
      next.delete(id);
      return { entries: next };
    }),
  list: () => Array.from(get().entries.values()),
  entriesByCombo: (combo) => Array.from(get().entries.values()).filter((e) => e.combo === combo),
}));
```

- [ ] **Step 4: Run, pass**

```bash
pnpm vitest run src/stores/keyboardStore
```

Expected: 4 PASS.

- [ ] **Step 5: Commit**

```bash
pnpm biome check .
git add src/stores/keyboardStore.ts src/stores/keyboardStore.test.ts
git commit -m "feat(keyboard): registry store (register/unregister/entriesByCombo)"
```

---

## Task 4: useKeyboardShortcut hook + global dispatcher + canonicalize

**Files:**
- Create: `src/hooks/useKeyboardShortcut.ts`
- Create: `src/hooks/useKeyboardShortcut.test.tsx`
- Create: `src/hooks/useGlobalKeyboardDispatch.ts`
- Create: `src/lib/canonicalCombo.ts`
- Create: `src/lib/canonicalCombo.test.ts`

- [ ] **Step 1: canonicalCombo helper test + impl**

```ts
// src/lib/canonicalCombo.test.ts
import { describe, expect, it } from "vitest";
import { eventToCombo } from "@/lib/canonicalCombo";

function ev(opts: Partial<KeyboardEventInit> & { key: string }) {
  return new KeyboardEvent("keydown", opts);
}

describe("eventToCombo", () => {
  it("plain letter", () => expect(eventToCombo(ev({ key: "a" }))).toBe("A"));
  it("Mod+Z (mac meta)", () =>
    expect(eventToCombo(ev({ key: "z", metaKey: true }))).toBe("Mod+Z"));
  it("Mod+Z (linux/win ctrl)", () =>
    expect(eventToCombo(ev({ key: "z", ctrlKey: true }))).toBe("Mod+Z"));
  it("Mod+Shift+Z", () =>
    expect(eventToCombo(ev({ key: "z", metaKey: true, shiftKey: true }))).toBe("Mod+Shift+Z"));
  it("Mod+/ uses literal slash", () =>
    expect(eventToCombo(ev({ key: "/", metaKey: true }))).toBe("Mod+/"));
  it("? maps to ?", () => expect(eventToCombo(ev({ key: "?" }))).toBe("?"));
  it("Mod+Enter", () =>
    expect(eventToCombo(ev({ key: "Enter", metaKey: true }))).toBe("Mod+Enter"));
  it("Mod+1", () => expect(eventToCombo(ev({ key: "1", metaKey: true }))).toBe("Mod+1"));
});
```

```ts
// src/lib/canonicalCombo.ts
/**
 * Canonicalize a KeyboardEvent into a stable string.
 * `Mod+` collapses macOS Meta + Linux/Windows Ctrl into one platform-agnostic
 * prefix so shortcut definitions don't have to branch.
 *
 * Letters are uppercased; Enter / Escape / numeric / punctuation pass through.
 */
export function eventToCombo(e: KeyboardEvent): string {
  const parts: string[] = [];
  if (e.metaKey || e.ctrlKey) parts.push("Mod");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  let key = e.key;
  if (key.length === 1 && key >= "a" && key <= "z") key = key.toUpperCase();
  parts.push(key);
  return parts.join("+");
}
```

- [ ] **Step 2: useKeyboardShortcut + dispatcher tests + impls**

```tsx
// src/hooks/useKeyboardShortcut.test.tsx
import { render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useGlobalKeyboardDispatch } from "@/hooks/useGlobalKeyboardDispatch";
import { useKeyboardShortcut } from "@/hooks/useKeyboardShortcut";
import { useKeyboardStore } from "@/stores/keyboardStore";

function Probe({ id, combo, handler, scope }: { id: string; combo: string; handler: () => void; scope: "global" | "page" }) {
  useKeyboardShortcut({ id, combo, handler, scope, label: id });
  return null;
}

function DispatcherProbe() {
  useGlobalKeyboardDispatch();
  return null;
}

describe("useKeyboardShortcut + dispatcher", () => {
  afterEach(() => {
    useKeyboardStore.setState({ entries: new Map() });
  });

  it("registers on mount, unregisters on unmount", () => {
    const { unmount } = render(<Probe id="x" combo="Mod+S" handler={() => {}} scope="global" />);
    expect(useKeyboardStore.getState().list()).toHaveLength(1);
    unmount();
    expect(useKeyboardStore.getState().list()).toHaveLength(0);
  });

  it("dispatcher fires the matching handler", () => {
    const handler = vi.fn();
    render(
      <>
        <DispatcherProbe />
        <Probe id="x" combo="Mod+S" handler={handler} scope="global" />
      </>,
    );
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "s", metaKey: true }));
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it("page scope wins over global for the same combo", () => {
    const global = vi.fn();
    const page = vi.fn();
    render(
      <>
        <DispatcherProbe />
        <Probe id="g" combo="Mod+Enter" handler={global} scope="global" />
        <Probe id="p" combo="Mod+Enter" handler={page} scope="page" />
      </>,
    );
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", metaKey: true }));
    expect(page).toHaveBeenCalledTimes(1);
    expect(global).not.toHaveBeenCalled();
  });

  it("text-field focus suppresses dispatch", () => {
    const handler = vi.fn();
    render(
      <>
        <DispatcherProbe />
        <Probe id="x" combo="Mod+Z" handler={handler} scope="global" />
        <textarea data-testid="ta" />
      </>,
    );
    const ta = document.querySelector('[data-testid="ta"]') as HTMLTextAreaElement;
    ta.focus();
    ta.dispatchEvent(new KeyboardEvent("keydown", { key: "z", metaKey: true, bubbles: true }));
    expect(handler).not.toHaveBeenCalled();
  });
});
```

```ts
// src/hooks/useKeyboardShortcut.ts
import { useEffect } from "react";
import { useKeyboardStore } from "@/stores/keyboardStore";
import type { ShortcutEntry } from "@/stores/keyboardStore";

/**
 * Register a keyboard shortcut for the lifetime of the component.
 *
 * Caller MUST memoize `handler` (useCallback) if it captures state — otherwise
 * the registry re-registers every render. The hook only re-syncs when `id`,
 * `combo`, or `scope` change.
 */
export function useKeyboardShortcut(entry: ShortcutEntry): void {
  const register = useKeyboardStore((s) => s.register);
  const unregister = useKeyboardStore((s) => s.unregister);

  useEffect(() => {
    register(entry);
    return () => unregister(entry.id);
    // biome-ignore lint/correctness/useExhaustiveDependencies: re-register on identity change of any field
  }, [entry, register, unregister]);
}
```

```ts
// src/hooks/useGlobalKeyboardDispatch.ts
import { useEffect } from "react";
import { eventToCombo } from "@/lib/canonicalCombo";
import { useKeyboardStore } from "@/stores/keyboardStore";

const SCOPE_PRIORITY: Record<string, number> = {
  page: 0,
  "module:website": 1,
  "module:graphic2d": 1,
  "module:graphic3d": 1,
  "module:video": 1,
  "module:typography": 1,
  global: 2,
};

function isTextField(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA") return true;
  if (target.isContentEditable) return true;
  return false;
}

/**
 * Mount once at the App root. Listens to document keydown events,
 * canonicalizes them, finds the highest-priority registered handler, and
 * dispatches. Suppresses dispatch in text-fields (so native text editing
 * undo / cursor movement still works).
 */
export function useGlobalKeyboardDispatch(): void {
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (isTextField(e.target)) return;
      const combo = eventToCombo(e);
      const matches = useKeyboardStore.getState().entriesByCombo(combo);
      if (matches.length === 0) return;
      const passing = matches.filter((m) => !m.when || m.when());
      if (passing.length === 0) return;
      passing.sort((a, b) => (SCOPE_PRIORITY[a.scope] ?? 99) - (SCOPE_PRIORITY[b.scope] ?? 99));
      const winner = passing[0];
      if (!winner) return;
      e.preventDefault();
      winner.handler();
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, []);
}
```

- [ ] **Step 3: Run all new tests; verify pass**

```bash
pnpm vitest run src/lib/canonicalCombo src/hooks/useKeyboardShortcut src/stores/keyboardStore
```

Expected: all PASS.

- [ ] **Step 4: Mount dispatcher in App.tsx**

In `src/App.tsx`, inside the App component body (alongside `useUndoRedo()`, `useBudgetPoll()`):

```tsx
import { useGlobalKeyboardDispatch } from "@/hooks/useGlobalKeyboardDispatch";
// ...
useGlobalKeyboardDispatch();
```

- [ ] **Step 5: Commit**

```bash
pnpm biome check .
git add src/lib/canonicalCombo.ts src/lib/canonicalCombo.test.ts src/hooks/useKeyboardShortcut.ts src/hooks/useKeyboardShortcut.test.tsx src/hooks/useGlobalKeyboardDispatch.ts src/App.tsx
git commit -m "feat(keyboard): useKeyboardShortcut + global dispatcher (scope-priority + text-field suppression)"
```

---

## Task 5: Migrate useUndoRedo onto the registry

**Files:**
- Modify: `src/hooks/useUndoRedo.ts`

- [ ] **Step 1: Verify the existing test is the regression gate**

```bash
pnpm vitest run src/hooks/useUndoRedo
```

Expected: PASS (current implementation).

- [ ] **Step 2: Refactor in-place**

```ts
// src/hooks/useUndoRedo.ts
import { useCallback } from "react";
import { useGlobalKeyboardDispatch } from "@/hooks/useGlobalKeyboardDispatch";
import { useKeyboardShortcut } from "@/hooks/useKeyboardShortcut";
import { useHistoryStore } from "@/stores/historyStore";

/**
 * Mount once (typically in `App`) to wire keyboard shortcuts to the history
 * store. macOS `⌘+Z` and Windows/Linux `Ctrl+Z` undo; adding `Shift` redoes.
 *
 * Implemented via the keyboard registry — text-field suppression and
 * cross-platform Mod handling come from the dispatcher.
 */
export function useUndoRedo(): void {
  const undo = useCallback(() => useHistoryStore.getState().undo(), []);
  const redo = useCallback(() => useHistoryStore.getState().redo(), []);

  useKeyboardShortcut({
    id: "global:undo",
    combo: "Mod+Z",
    handler: undo,
    scope: "global",
    label: "Undo",
  });
  useKeyboardShortcut({
    id: "global:redo",
    combo: "Mod+Shift+Z",
    handler: redo,
    scope: "global",
    label: "Redo",
  });

  // The dispatcher is mounted once in App.tsx, so this hook does NOT mount it
  // again. Kept import above for future readers — remove if grep shows
  // unused after lint pass.
  void useGlobalKeyboardDispatch;
}
```

Actually drop the `void useGlobalKeyboardDispatch;` line — leave a clean implementation:

```ts
// src/hooks/useUndoRedo.ts
import { useCallback } from "react";
import { useKeyboardShortcut } from "@/hooks/useKeyboardShortcut";
import { useHistoryStore } from "@/stores/historyStore";

/**
 * Mount once (typically in `App`) to wire Cmd/Ctrl+Z / Cmd/Ctrl+Shift+Z to
 * the history store. Implemented via the keyboard registry — cross-platform
 * Mod handling and text-field suppression come from the dispatcher.
 */
export function useUndoRedo(): void {
  const undo = useCallback(() => useHistoryStore.getState().undo(), []);
  const redo = useCallback(() => useHistoryStore.getState().redo(), []);

  useKeyboardShortcut({
    id: "global:undo",
    combo: "Mod+Z",
    handler: undo,
    scope: "global",
    label: "Undo",
  });
  useKeyboardShortcut({
    id: "global:redo",
    combo: "Mod+Shift+Z",
    handler: redo,
    scope: "global",
    label: "Redo",
  });
}
```

- [ ] **Step 3: Run existing test — must still pass**

```bash
pnpm vitest run src/hooks/useUndoRedo
```

Expected: PASS unchanged. The existing test mounts `useUndoRedo()` and dispatches Cmd+Z events. After this refactor it will need the dispatcher mounted too. **If the test fails, update it to also mount `useGlobalKeyboardDispatch()`** — that's the legitimate change required by the migration; preserve the assertion structure.

If the test needs updating:

```tsx
// src/hooks/useUndoRedo.test.tsx — only the Probe component changes
function Probe() {
  useUndoRedo();
  // The dispatcher is normally mounted in App.tsx; mount it here so the
  // registry-based shortcuts actually fire under jsdom.
  const { useGlobalKeyboardDispatch } = require("@/hooks/useGlobalKeyboardDispatch");
  useGlobalKeyboardDispatch();
  return null;
}
```

(Use proper `import` instead of `require` if the codebase doesn't use `require`. Verify by grep.)

- [ ] **Step 4: Commit**

```bash
pnpm biome check .
git add src/hooks/useUndoRedo.ts src/hooks/useUndoRedo.test.tsx
git commit -m "refactor(keyboard): migrate useUndoRedo onto useKeyboardShortcut registry"
```

---

## Task 6: Sidebar registers Cmd+1-5 + Cmd+N

**Files:**
- Modify: `src/components/shell/Sidebar.tsx`

- [ ] **Step 1: Find the modules list in Sidebar**

```bash
grep -n "MODULES\|module" src/components/shell/Sidebar.tsx | head -10
```

The Sidebar imports `MODULES` from `src/components/shell/modules.ts`. Each module has an `id`, `label`, `path`.

- [ ] **Step 2: Wire shortcuts**

In `Sidebar.tsx`, inside the component body (after existing hooks, before render):

```tsx
import { useNavigate } from "react-router-dom";
import { useKeyboardShortcut } from "@/hooks/useKeyboardShortcut";
// ...inside the component:
const navigate = useNavigate();

// Cmd+1 → first module, Cmd+2 → second, etc. Capped at 9.
MODULES.slice(0, 9).forEach((m, i) => {
  // biome-ignore lint/correctness/useHookAtTopLevel: stable iteration over a constant array
  useKeyboardShortcut({
    id: `global:module:${m.id}`,
    combo: `Mod+${i + 1}`,
    handler: () => navigate(m.path),
    scope: "global",
    label: m.label,
  });
});

// Cmd+N opens the New Project dialog (App.tsx exposes the open setter via
// a callback prop — verify by reading App.tsx and Sidebar.tsx; if no prop
// exists, add `onNewProject?: () => void` to SidebarProps and wire from App.)
```

The `forEach + biome-ignore` pattern violates the rules-of-hooks lint normally; the workaround is to enumerate explicitly:

```tsx
// Replace the forEach loop with explicit calls — MODULES order is stable.
useKeyboardShortcut({
  id: "global:module:website",
  combo: "Mod+1",
  handler: () => navigate("/website"),
  scope: "global",
  label: "Website Builder",
});
useKeyboardShortcut({
  id: "global:module:graphic2d",
  combo: "Mod+2",
  handler: () => navigate("/graphic2d"),
  scope: "global",
  label: "2D Graphic",
});
useKeyboardShortcut({
  id: "global:module:graphic3d",
  combo: "Mod+3",
  handler: () => navigate("/graphic3d"),
  scope: "global",
  label: "Pseudo-3D",
});
useKeyboardShortcut({
  id: "global:module:video",
  combo: "Mod+4",
  handler: () => navigate("/video"),
  scope: "global",
  label: "Video",
});
useKeyboardShortcut({
  id: "global:module:typography",
  combo: "Mod+5",
  handler: () => navigate("/typography"),
  scope: "global",
  label: "Typography",
});
```

- [ ] **Step 3: Cmd+N for new project**

If Sidebar already exposes `onNewProject`, wire:

```tsx
useKeyboardShortcut({
  id: "global:new-project",
  combo: "Mod+N",
  handler: () => onNewProject?.(),
  scope: "global",
  label: "New project",
  when: () => Boolean(onNewProject),
});
```

If Sidebar doesn't have a Cmd+N hook today, skip Cmd+N here and instead register it in App.tsx alongside the existing `setNewDialogOpen(true)` callback.

- [ ] **Step 4: Run E2E navigation spec to verify**

```bash
pnpm e2e e2e/tests/navigation.spec.ts
```

Existing assertions should still pass; verifying Cmd+1-5 happens in Task 12.

- [ ] **Step 5: Commit**

```bash
pnpm biome check .
git add src/components/shell/Sidebar.tsx src/App.tsx
git commit -m "feat(keyboard): register Cmd+1-5 module switches and Cmd+N (new project)"
```

---

## Task 7: ShortcutHelpOverlay + Cmd+/ trigger

**Files:**
- Create: `src/components/shell/ShortcutHelpOverlay.tsx`
- Create: `src/components/shell/ShortcutHelpOverlay.test.tsx`
- Modify: `src/App.tsx` (mount overlay + register `Mod+/` and `?`)

- [ ] **Step 1: Write failing test**

```tsx
// src/components/shell/ShortcutHelpOverlay.test.tsx
import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ShortcutHelpOverlay } from "@/components/shell/ShortcutHelpOverlay";
import { useKeyboardStore } from "@/stores/keyboardStore";

describe("ShortcutHelpOverlay", () => {
  afterEach(() => {
    useKeyboardStore.setState({ entries: new Map() });
  });

  it("renders nothing when closed", () => {
    render(<ShortcutHelpOverlay open={false} onClose={() => {}} />);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("lists registered shortcuts grouped by scope when open", () => {
    useKeyboardStore.getState().register({
      id: "g:undo",
      combo: "Mod+Z",
      handler: () => {},
      scope: "global",
      label: "Undo",
    });
    useKeyboardStore.getState().register({
      id: "g:redo",
      combo: "Mod+Shift+Z",
      handler: () => {},
      scope: "global",
      label: "Redo",
    });
    render(<ShortcutHelpOverlay open={true} onClose={() => {}} />);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText("Undo")).toBeInTheDocument();
    expect(screen.getByText("Redo")).toBeInTheDocument();
    expect(screen.getByText("Mod+Z")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Implement**

```tsx
// src/components/shell/ShortcutHelpOverlay.tsx
import { Modal } from "@/components/ui/Modal";
import type { ShortcutScope } from "@/stores/keyboardStore";
import { useKeyboardStore } from "@/stores/keyboardStore";

const SCOPE_LABEL: Record<ShortcutScope, string> = {
  global: "Global",
  page: "This page",
  "module:website": "Website Builder",
  "module:graphic2d": "2D Graphic",
  "module:graphic3d": "Pseudo-3D",
  "module:video": "Video",
  "module:typography": "Typography",
};

export interface ShortcutHelpOverlayProps {
  open: boolean;
  onClose: () => void;
}

export function ShortcutHelpOverlay({ open, onClose }: ShortcutHelpOverlayProps) {
  const entries = useKeyboardStore((s) => s.list());

  const groups = new Map<ShortcutScope, typeof entries>();
  for (const e of entries) {
    const arr = groups.get(e.scope) ?? [];
    arr.push(e);
    groups.set(e.scope, arr);
  }

  return (
    <Modal open={open} onClose={onClose} title="Keyboard shortcuts" maxWidth={520}>
      <div className="flex flex-col gap-4">
        {Array.from(groups.entries()).map(([scope, list]) => (
          <div key={scope} className="flex flex-col gap-2">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              {SCOPE_LABEL[scope]}
            </span>
            <ul className="flex flex-col gap-1">
              {list.map((e) => (
                <li
                  key={e.id}
                  className="flex items-center justify-between text-2xs text-neutral-dark-200"
                >
                  <span>{e.label}</span>
                  <kbd className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-1.5 py-0.5 font-mono text-2xs text-neutral-dark-300">
                    {e.combo}
                  </kbd>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </Modal>
  );
}
```

- [ ] **Step 3: Mount + register triggers in App.tsx**

```tsx
// In App.tsx
import { useState } from "react";
import { ShortcutHelpOverlay } from "@/components/shell/ShortcutHelpOverlay";
import { useKeyboardShortcut } from "@/hooks/useKeyboardShortcut";
// ...
const [helpOpen, setHelpOpen] = useState(false);

useKeyboardShortcut({
  id: "global:help",
  combo: "Mod+/",
  handler: () => setHelpOpen((v) => !v),
  scope: "global",
  label: "Keyboard shortcuts",
});
useKeyboardShortcut({
  id: "global:help-q",
  combo: "?",
  handler: () => setHelpOpen((v) => !v),
  scope: "global",
  label: "Keyboard shortcuts (?)",
});

// ...in JSX:
<ShortcutHelpOverlay open={helpOpen} onClose={() => setHelpOpen(false)} />
```

- [ ] **Step 4: Run tests**

```bash
pnpm vitest run src/components/shell/ShortcutHelpOverlay
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
pnpm biome check .
git add src/components/shell/ShortcutHelpOverlay.tsx src/components/shell/ShortcutHelpOverlay.test.tsx src/App.tsx
git commit -m "feat(keyboard): ShortcutHelpOverlay + Cmd+/ + ? triggers"
```

---

## Task 8: LoadingButton component + module sweep

**Files:**
- Create: `src/components/ui/LoadingButton.tsx`
- Create: `src/components/ui/LoadingButton.test.tsx`
- Modify: `src/pages/Typography.tsx`, `WebsiteBuilder.tsx`, `Graphic2D.tsx`, `Graphic3D.tsx`, `Video.tsx` — replace `busy ? "Generating…" : "Generate"` patterns

- [ ] **Step 1: Test**

```tsx
// src/components/ui/LoadingButton.test.tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { LoadingButton } from "@/components/ui/LoadingButton";

describe("LoadingButton", () => {
  it("shows label when not loading", () => {
    render(<LoadingButton>Generate</LoadingButton>);
    expect(screen.getByRole("button", { name: /Generate/i })).toBeInTheDocument();
  });

  it("disables button when loading", () => {
    render(<LoadingButton loading>Generate</LoadingButton>);
    expect(screen.getByRole("button")).toBeDisabled();
  });

  it("renders a spinner indicator when loading", () => {
    render(<LoadingButton loading>Generate</LoadingButton>);
    expect(screen.getByTestId("loading-spinner")).toBeInTheDocument();
  });

  it("does not invoke onClick while loading", async () => {
    const user = userEvent.setup();
    const onClick = vi.fn();
    render(
      <LoadingButton loading onClick={onClick}>
        Generate
      </LoadingButton>,
    );
    // disabled buttons don't fire click in userEvent
    await user.click(screen.getByRole("button")).catch(() => {});
    expect(onClick).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Implement**

```tsx
// src/components/ui/LoadingButton.tsx
import { Loader2 } from "lucide-react";
import { Button, type ButtonProps } from "@/components/ui/Button";

export interface LoadingButtonProps extends ButtonProps {
  /** When true, the button is disabled and shows a spinner next to its label. */
  loading?: boolean;
}

export function LoadingButton({ loading, disabled, children, ...rest }: LoadingButtonProps) {
  return (
    <Button {...rest} disabled={disabled || loading} aria-busy={loading || undefined}>
      {loading ? (
        <Loader2
          data-testid="loading-spinner"
          className="h-3 w-3 animate-spin"
          strokeWidth={1.5}
          aria-hidden="true"
        />
      ) : null}
      {children}
    </Button>
  );
}
```

- [ ] **Step 3: Run test**

```bash
pnpm vitest run src/components/ui/LoadingButton
```

Expected: 4 PASS.

- [ ] **Step 4: Sweep — replace `busy ? "X…" : "X"` patterns**

Grep first to find all sites:

```bash
grep -rn "busy ? \|vectorizing ? \|generating ?" src/pages/ src/components/ | grep -v ".test."
```

For each match, replace the existing `<Button>` with `<LoadingButton>`:

Example transformation in `src/pages/Typography.tsx`:
```tsx
// before
<Button variant="primary" onClick={handleGenerate} disabled={!prompt.trim() || busy}>
  {busy ? "Generating…" : "Generate 6 variants"}
</Button>

// after
<LoadingButton variant="primary" onClick={handleGenerate} disabled={!prompt.trim()} loading={busy}>
  Generate 6 variants
</LoadingButton>
```

Note: drop `busy` from the `disabled` prop since LoadingButton already disables when loading.

Apply the same transform in:
- `src/pages/Typography.tsx` (Generate button — already in TypographyHeader, so modify there)
- `src/components/typography/TypographyHeader.tsx` (the actual Generate button)
- `src/pages/Typography.tsx` Vectorize button (`vectorizing` state)
- `src/pages/WebsiteBuilder.tsx` Generate
- `src/pages/Graphic2D.tsx` Generate
- `src/pages/Graphic3D.tsx` Generate (if present)
- `src/pages/Video.tsx` Generate
- Any other `*ing…` pattern surfaced by grep

- [ ] **Step 5: Run all tests**

```bash
pnpm vitest run
```

Expected: full suite green; existing module tests adapt because `getByRole('button', { name: /Generate/i })` still matches.

- [ ] **Step 6: Biome + commit**

```bash
pnpm biome check .
git add src/components/ui/LoadingButton.tsx src/components/ui/LoadingButton.test.tsx src/pages/ src/components/
git commit -m "feat(ui): LoadingButton + sweep all 'X…/X' patterns across modules"
```

---

## Task 9: HelpIcon component + technical-parameter sweep

**Files:**
- Create: `src/components/ui/HelpIcon.tsx`
- Create: `src/components/ui/HelpIcon.test.tsx`
- Modify: `src/components/typography/TextLogoControls.tsx` (kerning), `src/components/typography/SvgEditor.tsx` or wherever Vectorize controls live (filter_speckle, corner_threshold, color_mode)

- [ ] **Step 1: Test**

```tsx
// src/components/ui/HelpIcon.test.tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { HelpIcon } from "@/components/ui/HelpIcon";

describe("HelpIcon", () => {
  it("renders the trigger glyph", () => {
    render(<HelpIcon content="explanation" />);
    expect(screen.getByLabelText(/help/i)).toBeInTheDocument();
  });

  it("shows tooltip content on hover", async () => {
    const user = userEvent.setup();
    render(<HelpIcon content="Drops clusters smaller than NxN px" />);
    await user.hover(screen.getByLabelText(/help/i));
    // Tooltip is shown via framer-motion AnimatePresence after openDelay (200ms by default).
    expect(
      await screen.findByText("Drops clusters smaller than NxN px", {}, { timeout: 1000 }),
    ).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Implement**

```tsx
// src/components/ui/HelpIcon.tsx
import { HelpCircle } from "lucide-react";
import type { ReactNode } from "react";
import { Tooltip, type TooltipSide } from "@/components/ui/Tooltip";

export interface HelpIconProps {
  content: ReactNode;
  side?: TooltipSide;
}

/**
 * A `?` glyph that shows a Tooltip on hover/focus. Pair with technical-
 * parameter labels (kerning, filter_speckle, etc.) to give just-in-time
 * help without cluttering the UI.
 */
export function HelpIcon({ content, side = "top" }: HelpIconProps) {
  return (
    <Tooltip content={content} side={side}>
      <button
        type="button"
        aria-label="Help"
        className="inline-flex h-3 w-3 items-center justify-center rounded-full text-neutral-dark-500 hover:text-neutral-dark-200"
      >
        <HelpCircle className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
      </button>
    </Tooltip>
  );
}
```

- [ ] **Step 3: Sweep technical-parameter sites**

In `src/components/typography/TextLogoControls.tsx`, near the Kerning slider label:
```tsx
import { HelpIcon } from "@/components/ui/HelpIcon";
// ...next to the Kerning <span>:
<span>Kerning {value.kerning > 0 ? `+${value.kerning}` : value.kerning}</span>
<HelpIcon content="Adjusts space between characters (px). Negative values tighten letters." />
```

In whichever component hosts the Vectorize controls (likely the page or a `VectorizerControls` component — find via grep):
```bash
grep -rn "filter_speckle\|corner_threshold" src/ | grep -v ".test." | grep -v "vectorizerCommands"
```

Add HelpIcons for:
- `filter_speckle`: "Drops connected pixel clusters smaller than NxN. Higher = cleaner, lower = preserves detail."
- `corner_threshold`: "Below this angle (degrees), corners simplify to straight segments."
- `color_mode`: "Color preserves the full palette; B&W flattens to binary (best for icons / line art)."
- For image-generation pages (Graphic2D / WebsiteBuilder if they expose them): `cfg_scale` ("Higher = stricter prompt adherence, lower = more creative drift"), `steps` ("More steps = finer detail, longer generation"), `seed` ("Locks the random seed so the same prompt yields the same output").

If a parameter doesn't have visible controls in the UI today (only present internally), skip it — HelpIcon only makes sense next to a visible control.

- [ ] **Step 4: Run tests**

```bash
pnpm vitest run src/components/ui/HelpIcon src/components/typography/
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
pnpm biome check .
git add src/components/ui/HelpIcon.tsx src/components/ui/HelpIcon.test.tsx src/components/ src/pages/
git commit -m "feat(ui): HelpIcon component + sweep on filter_speckle/corner_threshold/kerning/cfg_scale/steps/seed"
```

---

## Task 10: Tooltip sweep on icon-only buttons

**Files:**
- Modify: `src/components/typography/LogoGallery.tsx` (heart button)
- Modify: `src/components/shell/Sidebar.tsx` (module links)
- Other icon-only sites surfaced by grep

- [ ] **Step 1: Find icon-only buttons**

```bash
grep -rn "aria-label=" src/ | grep -v ".test." | grep -v "<input " | head -30
```

For each `aria-label="..."` on a `<button>` that has only an icon child (no text), wrap with `<Tooltip>`.

- [ ] **Step 2: LogoGallery heart**

In `src/components/typography/LogoGallery.tsx`, wrap the heart button:

```tsx
import { Tooltip } from "@/components/ui/Tooltip";
// ...
<Tooltip content={fav ? "Remove from favorites" : "Add to favorites"}>
  <button
    type="button"
    onClick={() => toggleFavorite(v.url)}
    aria-label={fav ? "Unfavorite" : "Favorite"}
    className="absolute top-1 right-1 rounded-full bg-neutral-dark-950/80 p-1"
  >
    <Heart className={`h-3 w-3 ${fav ? "fill-accent-500 text-accent-500" : "text-neutral-dark-400"}`} />
  </button>
</Tooltip>
```

- [ ] **Step 3: Sidebar module links**

In `Sidebar.tsx`, wrap each module link in a Tooltip showing the module name + Cmd+N hint (where N is the module's index):

```tsx
<Tooltip content={`${m.label} (Mod+${i + 1})`} side="right">
  <button
    type="button"
    onClick={() => navigate(m.path)}
    aria-label={m.label}
    className="..."
  >
    <Icon className="h-4 w-4" />
  </button>
</Tooltip>
```

- [ ] **Step 4: Run all tests, ensure no regressions**

```bash
pnpm vitest run
```

Existing LogoGallery + Sidebar tests must still pass — Tooltip doesn't change the trigger's role/aria.

- [ ] **Step 5: Commit**

```bash
pnpm biome check .
git add src/components/typography/LogoGallery.tsx src/components/shell/Sidebar.tsx
git commit -m "feat(ui): Tooltip sweep on icon-only buttons (LogoGallery heart, Sidebar modules)"
```

---

## Task 11: Skeleton tiles + Toast progress field

**Files:**
- Modify: `src/components/typography/LogoGallery.tsx` (Skeleton tiles when busy + empty)
- Modify: `src/stores/uiStore.ts` (add `progress?: { current: number; total: number }` to Notification + NotificationInput)
- Modify: `src/components/ui/Toast.tsx` (render bar when progress present)
- Modify: `src/pages/Typography.tsx` (pass `busy` prop down to LogoGallery)
- Add caller emitting progress: pick `src/lib/exporterCommands.ts` or `remotionCommands.ts` if either already polls; if neither does, add a sample progress emission to the Brand-kit export call

- [ ] **Step 1: LogoGallery skeleton**

In `src/components/typography/LogoGallery.tsx`, accept a new `busy?: boolean` prop. When `busy && variants.length === 0`, render 6 Skeleton tiles in a 3×2 grid:

```tsx
import { Skeleton } from "@/components/ui/Skeleton";

export interface LogoGalleryProps {
  variants: LogoVariant[];
  selectedUrl: string | null;
  onSelect: (url: string) => void;
  /** When true and no variants yet, render skeleton tiles. */
  busy?: boolean;
}

export function LogoGallery({ variants, selectedUrl, onSelect, busy }: LogoGalleryProps) {
  // ... existing favOnly + filter logic ...
  if (busy && variants.length === 0) {
    return (
      <div className="grid grid-cols-3 gap-2 p-3">
        {Array.from({ length: 6 }).map((_, i) => (
          <Skeleton
            // biome-ignore lint/suspicious/noArrayIndexKey: static placeholder list
            key={`skeleton-${i}`}
            className="aspect-square w-full"
          />
        ))}
      </div>
    );
  }
  // ... existing render ...
}
```

In `src/pages/Typography.tsx`, pass `busy={busy}` to `<LogoGallery />`.

- [ ] **Step 2: Toast progress wiring (test first)**

```tsx
// src/components/ui/Toast.test.tsx — extend existing tests
it("renders progress bar when progress is present", () => {
  useUiStore.getState().notify({
    kind: "info",
    message: "Rendering",
    progress: { current: 3, total: 10 },
  });
  render(<Toaster />);
  const bar = screen.getByTestId("toast-progress");
  expect(bar).toBeInTheDocument();
  // 30% → style width: 30%
  expect(bar).toHaveStyle({ width: "30%" });
});

it("does NOT render a progress bar when progress is absent", () => {
  useUiStore.getState().notify({ kind: "info", message: "Hello" });
  render(<Toaster />);
  expect(screen.queryByTestId("toast-progress")).not.toBeInTheDocument();
});
```

- [ ] **Step 3: Implement progress field**

In `src/stores/uiStore.ts`:

```ts
export interface Notification {
  id: string;
  kind: NotificationKind;
  message: string;
  detail?: string;
  createdAt: string;
  /** Optional progress for long-running operations: { current, total }. */
  progress?: { current: number; total: number };
}
```

(`NotificationInput` already infers from `Notification` via the existing `Omit<...>` type, so no change needed there.)

In `src/components/ui/Toast.tsx`, after the existing label/detail render, add (inside the `motion.div`):

```tsx
{n.progress ? (
  <div className="absolute right-0 bottom-0 left-0 h-0.5 overflow-hidden rounded-b-xs bg-neutral-dark-800">
    <div
      data-testid="toast-progress"
      className="h-full bg-accent-500 transition-[width] duration-200"
      style={{ width: `${Math.round((n.progress.current / Math.max(1, n.progress.total)) * 100)}%` }}
    />
  </div>
) : null}
```

- [ ] **Step 4: Auto-dismiss should NOT fire while progress < total**

In Toaster's auto-dismiss `useEffect`, gate dismissal by `progress`:

```tsx
for (const n of notifications) {
  // Don't auto-dismiss in-flight progress toasts; wait for caller to update them
  // to current === total or push a fresh terminal notification.
  if (n.progress && n.progress.current < n.progress.total) continue;
  timers.push(window.setTimeout(() => dismiss(n.id), autoDismissMs));
}
```

- [ ] **Step 5: Wire one real progress emission (sample)**

Pick the easiest existing caller. Option: `src/pages/Typography.tsx` `handleExport` shows a "Brand kit exported" success toast at the end — for now, that's already a single terminal toast and does NOT have intermediate progress. Adding intermediate progress to it would require backend events that don't exist.

**Pragmatic choice:** wire progress for the **vectorize-then-export multi-step flow in Typography**. The current toast emissions are "Vectorized logo" then later "Brand kit exported". Combine into a single notification with progress:

```tsx
// In handleExport, before exportBrandKit:
const progressId = notify({
  kind: "info",
  message: "Building brand kit",
  progress: { current: 0, total: 12 }, // 12 expected assets
});
// (No backend progress event today — leave the bar at 0% then push a terminal
// success toast on completion. The bar serves as a "this is happening" cue.)
const zipPath = await exportBrandKit(input, dialogInput.destination);
useUiStore.getState().dismissNotification(progressId);
notify({ kind: "success", message: "Brand kit exported", detail: zipPath });
```

This is acceptable: it doesn't pretend to know real progress, but it shows a visible bar instead of a frozen UI during the wait. **If you find a backend that DOES emit incremental events (Remotion render polling), wire it instead — that's stronger justification.**

- [ ] **Step 6: Run tests + commit**

```bash
pnpm vitest run src/components/typography/LogoGallery src/components/ui/Toast src/pages/Typography
pnpm biome check .
git add src/components/typography/LogoGallery.tsx src/stores/uiStore.ts src/components/ui/Toast.tsx src/components/ui/Toast.test.tsx src/pages/Typography.tsx
git commit -m "feat(ui): Skeleton tiles in LogoGallery + Toast progress field + brand-kit progress wiring"
```

---

## Task 12: E2E coverage — welcome.spec + navigation Cmd+1-5 extension

**Files:**
- Create: `e2e/tests/welcome.spec.ts`
- Modify: `e2e/tests/navigation.spec.ts` (add Cmd+1-5 assertion)

- [ ] **Step 1: welcome.spec**

```ts
// e2e/tests/welcome.spec.ts
import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

test.describe("Welcome onboarding", () => {
  test.beforeEach(async ({ page }) => {
    // Clear the localStorage flag so the modal opens
    await installInvokeMock(page, {
      list_projects: [],
      get_budget_status: { state: "ok", used_today_cents: 0, limits: { daily_cents: 0 }, day_started_at: "", session_started_at: "" },
      get_queue_status: { pending: 0, in_flight: 0 },
    });
    await page.addInitScript(() => {
      window.localStorage.removeItem("tm:welcome:dismissed");
    });
  });

  test("modal appears on first launch and dismisses on Done", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("dialog", { name: /Welcome to TERRYBLEMACHINE/i })).toBeVisible();
    await page.getByRole("button", { name: /Next/i }).click();
    await page.getByRole("button", { name: /Next/i }).click();
    await page.getByRole("button", { name: /Done/i }).click();
    await expect(page.getByRole("dialog", { name: /Welcome/i })).not.toBeVisible();

    // Reload — should NOT reappear (flag persists in this page's localStorage)
    await page.reload();
    await expect(page.getByRole("dialog", { name: /Welcome/i })).not.toBeVisible();
  });

  test("Skip dismisses without going through steps", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("dialog", { name: /Welcome/i })).toBeVisible();
    await page.getByRole("button", { name: /Skip/i }).click();
    await expect(page.getByRole("dialog", { name: /Welcome/i })).not.toBeVisible();
  });
});
```

- [ ] **Step 2: navigation.spec extension**

In `e2e/tests/navigation.spec.ts`, add a new test inside the existing describe:

```ts
test("Mod+1 through Mod+5 switch modules", async ({ page }) => {
  await page.goto("/");
  // Dismiss welcome if it pops (unit-setup default is dismissed; e2e default is dismissed too unless welcome.spec ran)
  await page.evaluate(() => window.localStorage.setItem("tm:welcome:dismissed", "true"));
  await page.reload();

  await page.keyboard.press("Meta+5");
  await expect(page).toHaveURL(/\/typography/);
  await page.keyboard.press("Meta+1");
  await expect(page).toHaveURL(/\/website/);
  await page.keyboard.press("Meta+2");
  await expect(page).toHaveURL(/\/graphic2d/);
});
```

(Note: in Playwright the macOS Meta key is `Meta`. On CI Ubuntu/Linux runners Playwright maps `Meta` to `Control` automatically when `os` is detected. Verify by running locally first.)

- [ ] **Step 3: Run E2E**

```bash
pnpm e2e
```

Expected: all 11 tests pass (9 existing + 2 new welcome + 1 new navigation extension = 12 total).

- [ ] **Step 4: Commit**

```bash
pnpm biome check .
git add e2e/tests/welcome.spec.ts e2e/tests/navigation.spec.ts
git commit -m "test(e2e): welcome onboarding spec + Cmd+1-5 module-switch in navigation"
```

---

## Task 13: Sub-Project verification + closure report

**Files:**
- Create: `docs/superpowers/specs/2026-04-19-phase-8-ux-polish-verification-report.md`

- [ ] **Step 1: Run full verification pipeline**

```bash
pnpm test
pnpm test:coverage
cd src-tauri && cargo test && cd ..
pnpm e2e
pnpm exec tsc --noEmit
pnpm biome check .
```

All MUST be green.

- [ ] **Step 2: Write report**

```markdown
# Phase 8 Sub-Project 2 (UX-Polish) — Verification Report

**Date:** 2026-04-19
**Spec:** `docs/superpowers/specs/2026-04-19-phase-8-ux-polish-design.md`
**Plan:** `docs/superpowers/plans/2026-04-19-phase-8-ux-polish.md`

## Summary

All 4 pillars implemented:
1. Welcome-Modal onboarding with 3 steps + localStorage skip flag
2. Keyboard-shortcut registry + Cmd+/ help overlay (useUndoRedo migrated; Cmd+1-5 + Cmd+N + Mod+Z/Mod+Shift+Z + ? wired)
3. Tooltips on icon-only buttons + HelpIcon on technical parameters (kerning, filter_speckle, corner_threshold, color_mode, cfg_scale, steps, seed)
4. LoadingButton replacing all `busy ? "X…" : "X"` patterns + Skeleton tiles in LogoGallery + Toast progress field

Verification pipeline: <numbers> frontend tests / <N> e2e specs / Rust unchanged / coverage / lint clean.

CI: latest run green across all 5 jobs.

## Pillar coverage

### 1. Onboarding — `src/components/onboarding/`, `src/hooks/useWelcomeFlow.ts`
### 2. Keyboard — `src/stores/keyboardStore.ts`, `src/hooks/useKeyboardShortcut.ts`, `src/hooks/useGlobalKeyboardDispatch.ts`, `src/components/shell/ShortcutHelpOverlay.tsx`, `src/lib/canonicalCombo.ts`. useUndoRedo migrated.
### 3. Tooltips/HelpIcon — `src/components/ui/HelpIcon.tsx` + sweep applied at <count> sites
### 4. LoadingButton/Skeleton/Progress — `src/components/ui/LoadingButton.tsx`, LogoGallery skeleton, Toast progress bar, Brand-kit-export progress emission

## Backlog filed during execution
<list any newly-filed backlog items here>

## Verdict

Phase 8 Sub-Project 2 (UX-Polish) closed. **Ready to brainstorm Sub-Project 3 (Performance).**
```

Replace `<numbers>`, `<N>`, `<count>`, `<list>` with REAL data.

- [ ] **Step 3: Commit + push + watch**

```bash
pnpm biome check .
git add docs/superpowers/specs/2026-04-19-phase-8-ux-polish-verification-report.md
git commit -m "docs(ux): Phase 8 Sub-Project 2 (UX-Polish) verification report"
git push origin main
gh run list --branch main --limit 1 --json databaseId
gh run watch <id> --exit-status
```

CI must be green.

---

## Self-Review

**Spec coverage:**
- ✓ Welcome-Modal onboarding → Tasks 1, 2
- ✓ Keyboard-shortcut registry → Tasks 3, 4
- ✓ useUndoRedo migration → Task 5
- ✓ Module shortcuts (Cmd+1-5 + Cmd+N) → Task 6
- ✓ ShortcutHelpOverlay (Cmd+/, ?) → Task 7
- ✓ LoadingButton + module sweep → Task 8
- ✓ HelpIcon + technical-parameter sweep → Task 9
- ✓ Tooltip on icon-only buttons → Task 10
- ✓ Skeleton tiles + Toast progress → Task 11
- ✓ E2E specs → Task 12
- ✓ Verification + closure → Task 13

**Placeholder scan:**
- Task 11 step 5 explicitly acknowledges "no backend progress event today" and uses a static 0%-bar as a "this is happening" cue. Documented as acceptable; alternate stronger justification (Remotion polling) noted as a switch path.
- Task 6 step 3 has a conditional ("If Sidebar already exposes onNewProject") with both branches specified. Not a placeholder.

**Type consistency:**
- `ShortcutEntry` defined in Task 3, used in Tasks 4 / 5 / 6 / 7 with same shape (id/combo/handler/scope/label/when?).
- `WELCOME_LOCALSTORAGE_KEY` exported from Task 1, imported in Tasks 2 / 12.
- `Notification.progress` defined in Task 11, consumed by Toast in same task.
- `LoadingButtonProps extends ButtonProps` in Task 8 — no shape drift in later tasks.

If during execution you find that `useKeyboardStore.list()` selector causes infinite re-renders (Zustand's `Map` referential identity issue), switch to `useKeyboardStore((s) => s.entries)` in `ShortcutHelpOverlay` and call `Array.from(entries.values())` inside render. Document that as a deviation if you have to apply it.
