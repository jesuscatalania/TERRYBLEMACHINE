# Phase 8 Sub-Project 3: Performance — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cut the initial JavaScript payload from 2.34 MB to under 600 KB by lazy-loading the 6 page components and splitting heavy vendor libs into on-demand chunks. Add a bundle visualizer so future drift stays visible.

**Architecture:** Two surgical pillars. (1) Replace eager page imports in `App.tsx` with `React.lazy()` + a `<Suspense>` boundary that renders a `<ModuleLoadingFallback />` skeleton while a chunk fetches. (2) Configure Vite's `build.rollupOptions.output.manualChunks` to split `three`, `fabric`, `monaco`, `framer-motion`, `react-dom + react-router-dom`, and other vendors into named chunks; add `rollup-plugin-visualizer` to emit `dist/stats.html` on every build.

**Tech Stack:** Vite 5+, React 19's `lazy` + `Suspense`, `rollup-plugin-visualizer` (new dev-dep). Reuses existing `<Skeleton />` primitive from Sub-Project 2.

**Source spec:** `docs/superpowers/specs/2026-04-19-phase-8-performance-design.md`

---

## File Structure

| File | Responsibility |
|---|---|
| `src/components/shell/ModuleLoadingFallback.tsx` | Skeleton-based fallback shown while a lazy module chunk is loading |
| `src/components/shell/ModuleLoadingFallback.test.tsx` | Renders + aria-busy assertion |
| `src/App.tsx` | Replaces 6 eager page imports with `lazy(...)`; wraps `<Routes>` in `<Suspense>` |
| `src/App.test.tsx` | Adapt 6 routing assertions from `getByText` to `findByText` (lazy resolution is async) |
| `vite.config.ts` | Adds `manualChunks` function + `visualizer` plugin |
| `package.json` | Adds `rollup-plugin-visualizer` to devDependencies |
| `docs/TESTING.md` | Append "Bundle inspection" section documenting chunks + how to read `stats.html` |
| `docs/superpowers/specs/2026-04-19-phase-8-performance-verification-report.md` | Sub-project closure |

---

## Task 1: ModuleLoadingFallback component

**Files:**
- Create: `src/components/shell/ModuleLoadingFallback.tsx`
- Create: `src/components/shell/ModuleLoadingFallback.test.tsx`

- [ ] **Step 1: Write failing test**

```tsx
// src/components/shell/ModuleLoadingFallback.test.tsx
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ModuleLoadingFallback } from "@/components/shell/ModuleLoadingFallback";

describe("ModuleLoadingFallback", () => {
  it("renders without crashing", () => {
    render(<ModuleLoadingFallback />);
    // Top-level wrapper should be present and aria-busy
    const wrapper = screen.getByRole("status");
    expect(wrapper).toBeInTheDocument();
    expect(wrapper).toHaveAttribute("aria-busy", "true");
  });

  it("contains skeleton placeholders", () => {
    const { container } = render(<ModuleLoadingFallback />);
    const skeletons = container.querySelectorAll("[data-skeleton='true']");
    expect(skeletons.length).toBeGreaterThanOrEqual(4); // header tag + 2 inputs + button + content
  });
});
```

- [ ] **Step 2: Run, verify FAIL**

```bash
pnpm vitest run src/components/shell/ModuleLoadingFallback
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement**

```tsx
// src/components/shell/ModuleLoadingFallback.tsx
import { Skeleton } from "@/components/ui/Skeleton";

/**
 * Fallback shown by `<Suspense>` while a lazy module chunk loads. Mirrors
 * the module shell layout (header tag + brief-row inputs + content area)
 * so the user sees a familiar shape rather than a flashing spinner.
 */
export function ModuleLoadingFallback() {
  return (
    <div
      className="grid h-full grid-rows-[auto_1fr]"
      role="status"
      aria-busy="true"
      aria-live="polite"
    >
      <div className="flex flex-col gap-3 border-neutral-dark-700 border-b p-6">
        <Skeleton width={140} height={12} />
        <div className="flex items-end gap-2">
          <Skeleton className="flex-1" height={36} />
          <Skeleton width={120} height={36} />
          <Skeleton width={120} height={36} />
        </div>
      </div>
      <div className="p-6">
        <Skeleton className="h-full w-full" />
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Run, verify PASS**

```bash
pnpm vitest run src/components/shell/ModuleLoadingFallback
```

Expected: 2 PASS.

- [ ] **Step 5: Biome + commit**

```bash
pnpm biome check .
git add src/components/shell/ModuleLoadingFallback.tsx src/components/shell/ModuleLoadingFallback.test.tsx
git commit -m "feat(perf): ModuleLoadingFallback skeleton for lazy-route Suspense"
```

---

## Task 2: Lazy-load 6 page components in App.tsx

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/App.test.tsx`

- [ ] **Step 1: Replace eager imports with lazy**

Open `src/App.tsx`. Locate lines 13-20 (the six page imports). Replace:

```tsx
// before:
import { DesignSystemPage } from "@/pages/DesignSystem";
import { Graphic2DPage } from "@/pages/Graphic2D";
import { Graphic3DPage } from "@/pages/Graphic3D";
import { TypographyPage } from "@/pages/Typography";
import { VideoPage } from "@/pages/Video";
import { WebsiteBuilderPage } from "@/pages/WebsiteBuilder";

// after:
import { lazy, Suspense } from "react";
import { ModuleLoadingFallback } from "@/components/shell/ModuleLoadingFallback";

const DesignSystemPage = lazy(() =>
  import("@/pages/DesignSystem").then((m) => ({ default: m.DesignSystemPage })),
);
const Graphic2DPage = lazy(() =>
  import("@/pages/Graphic2D").then((m) => ({ default: m.Graphic2DPage })),
);
const Graphic3DPage = lazy(() =>
  import("@/pages/Graphic3D").then((m) => ({ default: m.Graphic3DPage })),
);
const TypographyPage = lazy(() =>
  import("@/pages/Typography").then((m) => ({ default: m.TypographyPage })),
);
const VideoPage = lazy(() => import("@/pages/Video").then((m) => ({ default: m.VideoPage })));
const WebsiteBuilderPage = lazy(() =>
  import("@/pages/WebsiteBuilder").then((m) => ({ default: m.WebsiteBuilderPage })),
);
```

If the existing `import { useState }` line is already at the top, MERGE the new `lazy, Suspense` imports into that import statement instead of duplicating: `import { lazy, Suspense, useState } from "react";` (and remove any prior duplicate `useState` import).

- [ ] **Step 2: Wrap `<Routes>` in `<Suspense>`**

Inside `AnimatedRoutes`, wrap the existing `<Routes>` block:

```tsx
// before:
<Routes location={location}>
  <Route path="/" element={<Navigate to="/website" replace />} />
  ...
</Routes>

// after:
<Suspense fallback={<ModuleLoadingFallback />}>
  <Routes location={location}>
    <Route path="/" element={<Navigate to="/website" replace />} />
    ...
  </Routes>
</Suspense>
```

The `<motion.div>` parent stays — Suspense lives INSIDE the motion wrapper so the fallback animates in with the route transition.

- [ ] **Step 3: Adapt App.test.tsx routing assertions**

Existing tests use `screen.getByText(...)` for content that now resolves asynchronously after the lazy chunk loads. Convert each blocking `getByText` for module content to `findByText`:

```tsx
// before:
it("redirects / → /website and renders the Website builder", async () => {
  renderAt("/");
  expect(await screen.findByText(/WEBSITE BUILDER/)).toBeInTheDocument();  // already async — leave
  expect(useAppStore.getState().activeModule).toBe("website");
});

it("renders the Typography page at /typography", () => {  // sync — must change to async
  renderAt("/typography");
  expect(screen.getByText(/MOD—05 · TYPE & LOGO/)).toBeInTheDocument();
  expect(useAppStore.getState().activeModule).toBe("typography");
});
```

Change every `it(... , () => {` to `it(... , async ({ ... }) => {` AND every `screen.getByText(/MOD—...|WEBSITE BUILDER|...)/` for module content to `await screen.findByText(...)`.

Concrete transformation for each test:

```tsx
it("renders the Typography page at /typography", async () => {
  renderAt("/typography");
  expect(await screen.findByText(/MOD—05 · TYPE & LOGO/)).toBeInTheDocument();
  expect(useAppStore.getState().activeModule).toBe("typography");
});

it("renders the Website builder at /website", async () => {
  renderAt("/website");
  expect(await screen.findByText(/WEBSITE BUILDER/)).toBeInTheDocument();
  expect(await screen.findByLabelText(/Describe the site/i)).toBeInTheDocument();
});

it("renders the design system page at /design-system", async () => {
  renderAt("/design-system");
  expect(await screen.findByRole("heading", { name: /^design system$/i })).toBeInTheDocument();
});

it("renders the shell on every route", async () => {
  renderAt("/video");
  expect(await screen.findByText("TERRYBLEMACHINE")).toBeInTheDocument();
  // contentinfo (the footer) is part of the shell, NOT lazy — keep sync
  expect(screen.getByRole("contentinfo")).toBeInTheDocument();
});
```

(The "TERRYBLEMACHINE" brand text comes from the Sidebar — part of the always-mounted shell — so it could stay sync. But after lazy + Suspense the test re-renders during fallback transitions; using `findByText` is the safe default.)

- [ ] **Step 4: Run all App tests + dependent suites**

```bash
pnpm vitest run src/App
```

Expected: all 6 tests PASS. If any fail because the Suspense fallback's `aria-busy` interferes with a query, scope queries to the route-rendered DOM by also waiting for `aria-busy="true"` to disappear:

```tsx
await waitFor(() => {
  expect(screen.queryByRole("status", { busy: true })).not.toBeInTheDocument();
});
```

Add the import: `import { waitFor } from "@testing-library/react";` if needed.

- [ ] **Step 5: Run full vitest suite**

```bash
pnpm vitest run
```

Expected: all 380+ tests pass. If a non-App test fails because some component now relies on a lazy-loaded module mid-render, that's a real bug — investigate.

- [ ] **Step 6: Sanity-check the build**

```bash
pnpm build 2>&1 | tail -40
```

Expected: Vite output shows ~6 new per-route chunks (one per page) plus the existing main chunk. The main chunk is still large at this stage (vendor splits land in Task 3). Don't worry about the size yet — just verify the build succeeds and per-page chunks appear in `dist/assets/`.

- [ ] **Step 7: Biome + commit**

```bash
pnpm biome check .
git add src/App.tsx src/App.test.tsx
git commit -m "perf(app): lazy-load 6 page components with Suspense fallback"
```

---

## Task 3: Vite manualChunks vendor split

**Files:**
- Modify: `vite.config.ts`

- [ ] **Step 1: Add `manualChunks` function to build.rollupOptions**

In `vite.config.ts`, add a `build` block (the file currently has no `build` config). Insert after the `server` block, before `test`:

```ts
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes("node_modules")) {
            // Three.js + react-three ecosystem (Graphic3D module)
            if (/node_modules\/(three|@react-three\/(fiber|drei|postprocessing))/.test(id)) {
              return "vendor-three";
            }
            // Fabric.js (Graphic2D + Typography modules)
            if (/node_modules\/fabric\//.test(id)) return "vendor-fabric";
            // Monaco editor (WebsiteBuilder module)
            if (/node_modules\/@monaco-editor\//.test(id) || /node_modules\/monaco-editor\//.test(id)) {
              return "vendor-monaco";
            }
            // framer-motion (used by AnimatePresence, Toast, Tooltip — eager)
            if (/node_modules\/framer-motion/.test(id)) return "vendor-motion";
            // react-dom + router (eager but heavy enough to warrant own chunk)
            if (/node_modules\/(react-dom|react-router|react-router-dom)/.test(id)) {
              return "vendor-react";
            }
            // Everything else (lucide-react, zustand, gif.js, jsPDF, etc.)
            return "vendor-misc";
          }
        },
      },
    },
  },
```

`react` itself stays in the entry chunk (Vite's default — `manualChunks` only fires for non-entry modules; the main entry pulls react in directly).

- [ ] **Step 2: Build and inspect the output**

```bash
pnpm build 2>&1 | tail -25
```

Expected: output shows new chunks like `vendor-three-*.js`, `vendor-fabric-*.js`, `vendor-monaco-*.js`, `vendor-motion-*.js`, `vendor-react-*.js`, `vendor-misc-*.js`. The main `index-*.js` chunk should drop dramatically — target < 200 KB minified.

If a chunk lands in the wrong place (e.g., Three ends up in `vendor-misc`), refine the regex. Check the build output line-by-line; rg the chunk file for the unexpected lib name to confirm.

- [ ] **Step 3: Verify the entry shell still loads first**

The main entry chunk MUST contain react + react-dom-bound shell render code. If `vendor-react` ends up being a hard prerequisite that delays first paint, the user sees a blank screen. Verify by:

```bash
ls -lh dist/assets/index-*.js dist/assets/vendor-*.js
```

Both `index-*.js` and `vendor-react-*.js` should be reasonable sizes (entry < 200 KB; vendor-react < 150 KB). If the entry is empty or vendor-react balloons past 200 KB, the split is wrong — react is being deduped into vendor-react. Move `react-dom` out of `vendor-react` (just leave `react-router*` there) to keep react-dom in the entry:

```ts
// Modified — only react-router goes to vendor-react:
if (/node_modules\/react-router/.test(id)) return "vendor-react";
```

- [ ] **Step 4: Run full vitest suite**

```bash
pnpm vitest run
```

Expected: PASS. Vite's `manualChunks` doesn't affect dev or test (those use a different bundler path).

- [ ] **Step 5: Biome + commit**

```bash
pnpm biome check .
git add vite.config.ts
git commit -m "perf(build): manualChunks split (vendor-three/fabric/monaco/motion/react/misc)"
```

---

## Task 4: rollup-plugin-visualizer dev-dep + integration

**Files:**
- Modify: `package.json` (add devDep)
- Modify: `vite.config.ts` (add plugin)

- [ ] **Step 1: Install the visualizer**

```bash
pnpm add -D rollup-plugin-visualizer
```

This adds it under devDependencies. Capture the version installed (likely 5.x or 6.x).

- [ ] **Step 2: Add the plugin to vite.config.ts**

Add the import at the top:

```ts
import { visualizer } from "rollup-plugin-visualizer";
```

Insert into the `plugins` array (after `react()`):

```ts
plugins: [
  react(),
  visualizer({
    filename: "dist/stats.html",
    template: "treemap",
    gzipSize: true,
    brotliSize: false,
    open: false,
  }),
],
```

The plugin is always-on. `dist/` is already in `.gitignore` so `stats.html` won't be committed.

- [ ] **Step 3: Build + verify stats.html appears**

```bash
pnpm build 2>&1 | tail -10
ls -lh dist/stats.html
```

Expected: `dist/stats.html` exists, ~500KB-1MB (HTML report with embedded data). Open in a browser: `open dist/stats.html`. The treemap should show `vendor-three`, `vendor-fabric`, etc. as distinct boxes.

- [ ] **Step 4: Run full vitest suite (sanity)**

```bash
pnpm vitest run
```

The visualizer runs only during `build`, so unit tests are unaffected.

- [ ] **Step 5: Biome + commit**

```bash
pnpm biome check .
git add package.json pnpm-lock.yaml vite.config.ts
git commit -m "feat(build): rollup-plugin-visualizer emits dist/stats.html on every build"
```

---

## Task 5: docs/TESTING.md "Bundle inspection" section

**Files:**
- Modify: `docs/TESTING.md`

- [ ] **Step 1: Append the new section**

Open `docs/TESTING.md`. After the last existing section ("Manual QA"), append:

```markdown
## Bundle Inspection

After `pnpm build`, open `dist/stats.html` to see the chunk treemap (gzipped sizes). The build is split into:

| Chunk | Contents |
|---|---|
| `index-*.js` (entry) | App shell, Sidebar, Welcome modal, Toast, stores, react + react-dom |
| `vendor-three-*.js` | three.js + @react-three/{fiber,drei,postprocessing} (Graphic3D module only) |
| `vendor-fabric-*.js` | fabric.js (Graphic2D + Typography modules) |
| `vendor-monaco-*.js` | @monaco-editor/react + monaco-editor (WebsiteBuilder module) |
| `vendor-motion-*.js` | framer-motion (AnimatePresence, Toast, Tooltip animations) |
| `vendor-react-*.js` | react-router-dom (route-driven navigation) |
| `vendor-misc-*.js` | lucide-react, zustand, gif.js, etc. |
| Per-page chunks | Each `src/pages/*.tsx` lazy-loaded on first route entry |

Module pages load on demand the first time their route is entered (a brief `<ModuleLoadingFallback />` skeleton appears). Switching back to a previously-loaded module is instant.

If the main entry chunk grows past ~200 KB minified, inspect `dist/stats.html` to find the new heavy import — usually a vendor lib that needs adding to `vite.config.ts`'s `manualChunks`.
```

- [ ] **Step 2: Biome + commit**

```bash
pnpm biome check .
git add docs/TESTING.md
git commit -m "docs(testing): bundle inspection section (chunks + stats.html)"
```

---

## Task 6: Sub-Project verification + closure report

**Files:**
- Create: `docs/superpowers/specs/2026-04-19-phase-8-performance-verification-report.md`

- [ ] **Step 1: Run full verification pipeline**

```bash
pnpm test
pnpm test:coverage
cd src-tauri && cargo test && cd ..
pnpm e2e
pnpm exec tsc --noEmit
pnpm biome check .
pnpm build  # capture chunk sizes
```

All MUST be clean / green. Capture from the `pnpm build` output:
- New main entry chunk size (minified + gzipped)
- Sizes of vendor-* chunks
- Per-page chunk sizes (each `pages/*.tsx` lazy-loaded)

- [ ] **Step 2: Write closure report**

```markdown
# Phase 8 Sub-Project 3 (Performance) — Verification Report

**Date:** 2026-04-19
**Spec:** `docs/superpowers/specs/2026-04-19-phase-8-performance-design.md`
**Plan:** `docs/superpowers/plans/2026-04-19-phase-8-performance.md`

## Summary

Lazy-loading + manual chunk splits delivered. Bundle reshape:

**Before (commit `7fa3a76`):**
- main `index-*.js`: 2,341 kB minified / 688 kB gzipped (Vite warning)
- 3 split chunks: html2canvas, jsPDF, purify

**After:**
- main `index-*.js`: <SIZE> kB minified / <SIZE> kB gzipped
- vendor-three: <SIZE> kB
- vendor-fabric: <SIZE> kB
- vendor-monaco: <SIZE> kB
- vendor-motion: <SIZE> kB
- vendor-react: <SIZE> kB
- vendor-misc: <SIZE> kB
- 6 per-page chunks: <SIZE> kB each (avg)

Vite's "chunks larger than 500 kB" warning: <gone | persists for vendor-monaco only>.

Verification pipeline:
- Frontend: `pnpm test` <N> tests / <M> files
- Frontend coverage: <X>% lines / <Y>% branches
- Backend: `cargo test` <N> tests passing / <M> ignored (unchanged — frontend-only sub-project)
- E2E: `pnpm e2e` 12 tests across 7 spec files
- Lint: `pnpm biome check .` clean (<N> files); `pnpm exec tsc --noEmit` clean
- CI: latest run green across all 5 jobs

## Pillar coverage

### 1. Lazy-loading
- 6 page components (`WebsiteBuilder`, `Graphic2D`, `Graphic3D`, `Video`, `Typography`, `DesignSystem`) wrapped in `React.lazy(() => import(...).then(m => ({ default: m.X })))`
- `<Suspense fallback={<ModuleLoadingFallback />}>` mounted inside `<AnimatedRoutes>` so route transitions still animate
- `ModuleLoadingFallback` mirrors the module shell (header tag + brief-row inputs + content area) via existing `<Skeleton />` primitive

### 2. Vendor splits + visualizer
- `vite.config.ts` adds `build.rollupOptions.output.manualChunks` function with regex matchers for three, fabric, monaco, framer-motion, react-router; everything else lands in `vendor-misc`
- `rollup-plugin-visualizer` emits `dist/stats.html` (treemap, gzip sizes) on every build
- `docs/TESTING.md` documents chunks + how to inspect

## Backlog filed during execution
<list any newly-filed backlog items here, OR write "none">

## Verdict

Phase 8 Sub-Project 3 (Performance) closed. **All three Phase 8 sub-projects done.** Distribution remains explicitly out — revisited only after first live-test of the app.
```

Replace `<SIZE>`, `<N>`, `<M>`, `<X>`, `<Y>` with REAL numbers from your `pnpm build` and verify-pipeline runs.

- [ ] **Step 3: Commit + push + watch**

```bash
pnpm biome check .
git add docs/superpowers/specs/2026-04-19-phase-8-performance-verification-report.md
git commit -m "docs(perf): Phase 8 Sub-Project 3 (Performance) verification report"
git push origin main
gh run list --branch main --limit 1 --json databaseId
gh run watch <id> --exit-status
```

CI must be 5/5 green.

---

## Self-Review

**Spec coverage:**
- ✓ Lazy-loading 6 pages → Tasks 1, 2
- ✓ Vite manualChunks vendor splits → Task 3
- ✓ rollup-plugin-visualizer → Task 4
- ✓ docs/TESTING.md "Bundle inspection" → Task 5
- ✓ Acceptance criteria (main chunk < 600 KB, stats.html, all tests green) → Task 6

**Placeholder scan:**
- Task 6 closure report uses `<SIZE>` / `<N>` placeholders that the implementer MUST replace with real numbers — explicit in step 2. Not a plan failure (concrete substitutes are unknowable until the build runs).
- Task 3 Step 3 has a contingency ("If `vendor-react` ends up being a hard prerequisite...") with concrete fix code. Not a placeholder — both branches resolved.
- No "TBD" / "implement later" / vague "handle errors" anywhere.

**Type consistency:**
- `ModuleLoadingFallback` exported from Task 1 and imported in Task 2 — same name, named export.
- Skeleton's prop interface (width/height/className) used consistently.
- All page component names match `src/pages/*.tsx` named exports (WebsiteBuilderPage, Graphic2DPage, Graphic3DPage, VideoPage, TypographyPage, DesignSystemPage).
- Vite config: `manualChunks` is a function, returns string chunk names; `rollup-plugin-visualizer` plugin shape matches its v5+ API (filename / template / gzipSize / brotliSize / open).
