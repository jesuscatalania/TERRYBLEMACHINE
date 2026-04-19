# Phase 8 — Sub-Project 3: Performance (Design)

**Date:** 2026-04-19
**Phase context:** Phase 8 ("Polish & Release") decomposed into Testing → UX-Polish → Performance. Sub-Projects 1+2 closed. This is Sub-Project 3.
**Source plan:** `docs/ENTWICKLUNGSPLAN.md` lines 359-363 (Phase 8.1)

## Goal

Cut the initial JavaScript payload from 2.34 MB to under 600 KB by lazy-loading the 5 module pages and splitting heavy vendor libs (Three / Fabric / Monaco) into on-demand chunks. Add a bundle visualizer so future drift is visible.

## Baseline (measured 2026-04-19)

```
dist/assets/index-B23faNi8.js   2,341.44 kB │ gzip: 688.48 kB  ← main chunk (Vite warning)
dist/assets/html2canvas.esm     201.04 kB │ gzip:  47.43 kB    ← already split (jsPDF dep)
dist/assets/index.es (jsPDF)    158.79 kB │ gzip:  53.02 kB    ← already split
dist/assets/purify.es            22.71 kB │ gzip:   8.72 kB    ← already split
```

All 5 module pages + Three.js + Fabric.js + Monaco-Editor + framer-motion + react-three/* are eagerly imported in `src/App.tsx` and bundled into the single index chunk. Vite emits the "Some chunks are larger than 500 kB" warning on every build.

## Scope decisions (from brainstorming)

1. **Lazy-loading** the 6 page components (`WebsiteBuilder`, `Graphic2D`, `Graphic3D`, `Video`, `Typography`, `DesignSystem`) via `React.lazy()` + `<Suspense>`
2. **Vite `manualChunks`** for heavy vendors: `vendor-three`, `vendor-fabric`, `vendor-monaco`, `vendor-motion`, `vendor-react`, `vendor-misc`
3. **Bundle-visualizer dev-dep** (`rollup-plugin-visualizer`) emits `dist/stats.html` on every build
4. **Suspense fallback** is a `<ModuleLoadingFallback />` skeleton that mirrors the module shell layout (header tag + brief-row inputs + content area) — reuses the existing `<Skeleton />` primitive shipped in Sub-Project 2

## Non-Goals

- **Web Workers** — no UI-blocking hotspot identified today (Fabric `loadSVGFromString`, vectorize render, and gif.js are already async or worker-based). Filing as backlog: revisit after Tauri profiling identifies a real hot spot.
- **Rust profiling / `cargo flamegraph`** — backend pipelines already use `spawn_blocking` for CPU-bound work; the dominant latency is network (API calls) which profiling can't fix. Backlog.
- **Lighthouse CI / perf-budget gates** — Vite's chunk-size warning + the visualizer treemap are sufficient discovery tools for now. Hard gates in CI = backlog.
- **Image optimization** (WebP, responsive `srcset`) — out of scope; modules render raster outputs from the backend at native size.
- **Service Worker / offline cache** — Tauri ships static assets locally; no service-worker need.
- **Tauri binary stripping** — out of Phase 8 entirely (Distribution scope, deferred to post-live-test).
- **Splitting jsPDF / html2canvas / purify further** — already auto-split by Vite via dynamic imports inside their consumer modules. No additional work needed.

## Architecture — Where Things Live

| Component | Location |
|---|---|
| Lazy page imports | `src/App.tsx` — 6 imports become `const X = lazy(() => import("@/pages/X"))` |
| Suspense boundary | `src/App.tsx` — wraps `<AnimatedRoutes>` (or innermost `<Routes>`) with `<Suspense fallback={<ModuleLoadingFallback />}>` |
| Loading fallback | `src/components/shell/ModuleLoadingFallback.tsx` + test |
| Vite chunk strategy | `vite.config.ts` — adds `build.rollupOptions.output.manualChunks` function |
| Bundle visualizer | `vite.config.ts` — adds `rollup-plugin-visualizer` plugin (always-on; emits to `dist/stats.html`) |
| Doc updates | `docs/TESTING.md` — append "Bundle inspection" section pointing at the visualizer + the chunk strategy |
| Closure | `docs/superpowers/specs/2026-04-19-phase-8-performance-verification-report.md` |

## Components in Detail

### 1. Lazy-loading the 6 pages

Replace at the top of `src/App.tsx`:

```ts
import { lazy, Suspense } from "react";
const WebsiteBuilderPage = lazy(() => import("@/pages/WebsiteBuilder").then((m) => ({ default: m.WebsiteBuilderPage })));
const Graphic2DPage = lazy(() => import("@/pages/Graphic2D").then((m) => ({ default: m.Graphic2DPage })));
const Graphic3DPage = lazy(() => import("@/pages/Graphic3D").then((m) => ({ default: m.Graphic3DPage })));
const VideoPage = lazy(() => import("@/pages/Video").then((m) => ({ default: m.VideoPage })));
const TypographyPage = lazy(() => import("@/pages/Typography").then((m) => ({ default: m.TypographyPage })));
const DesignSystemPage = lazy(() => import("@/pages/DesignSystem").then((m) => ({ default: m.DesignSystemPage })));
```

(The `.then((m) => ({ default: m.X }))` shim is required because the page components are NAMED exports, but `React.lazy` expects a `{ default }` shape.)

Wrap the `<Routes>` rendering in `Suspense`:

```tsx
<Suspense fallback={<ModuleLoadingFallback />}>
  <Routes location={location}>
    {/* ...existing routes */}
  </Routes>
</Suspense>
```

The `<AnimatePresence>` parent stays — framer-motion correctly waits for Suspense to resolve before measuring layout. If a glitch surfaces, mount Suspense INSIDE the `<motion.div>` so the fallback animates with the page.

### 2. ModuleLoadingFallback

Skeleton layout that mimics the module shell (header tag + 2 input-shaped skeletons + button-shaped skeleton + content placeholder):

```tsx
import { Skeleton } from "@/components/ui/Skeleton";

export function ModuleLoadingFallback() {
  return (
    <div className="grid h-full grid-rows-[auto_1fr]" aria-busy="true" aria-live="polite">
      <div className="flex flex-col gap-3 border-neutral-dark-700 border-b p-6">
        <Skeleton width={140} height={12} />
        <div className="flex items-end gap-2">
          <Skeleton width="100%" height={36} className="flex-1" />
          <Skeleton width={120} height={36} />
          <Skeleton width={120} height={36} />
        </div>
      </div>
      <div className="p-6">
        <Skeleton width="100%" height="100%" />
      </div>
    </div>
  );
}
```

One unit test: renders without crashing + has `aria-busy="true"`.

### 3. Vite manualChunks

```ts
// vite.config.ts (excerpt — extend the existing build.rollupOptions.output)
export default defineConfig(async () => ({
  // ...existing top-level config
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes("node_modules")) {
            if (/three|@react-three\/(fiber|drei|postprocessing)/.test(id)) return "vendor-three";
            if (/\/fabric\//.test(id)) return "vendor-fabric";
            if (/monaco/.test(id)) return "vendor-monaco";
            if (/framer-motion/.test(id)) return "vendor-motion";
            if (/react-dom|react-router-dom/.test(id)) return "vendor-react";
            return "vendor-misc";
          }
        },
      },
    },
  },
  // ...rest stays
}));
```

Notes:
- `react` itself stays in the entry chunk (NOT in `vendor-react`) so the shell can render before any vendor chunk loads. Vite's default behavior already puts entry deps in the main chunk; the `manualChunks` function only fires for non-entry modules.
- `react-dom` and `react-router-dom` are heavy enough to warrant their own chunk, even though they load eagerly with the shell.
- `@react-three/postprocessing` matched by regex even though it has a `/` — explicit alternation.
- `\/fabric\/` (with surrounding slashes) avoids false matches on words containing "fabric" elsewhere.

### 4. Bundle-visualizer

Add `rollup-plugin-visualizer` as a dev dep. Configure once in `vite.config.ts`:

```ts
import { visualizer } from "rollup-plugin-visualizer";

export default defineConfig(async () => ({
  plugins: [
    react(),
    visualizer({
      filename: "dist/stats.html",
      template: "treemap",
      gzipSize: true,
      brotliSize: false,
      open: false, // never auto-open in CI
    }),
  ],
  // ...
}));
```

`dist/stats.html` lands in the build output. Add it to `.gitignore` if not covered by `dist/` (already covered).

### 5. Doc — TESTING.md "Bundle inspection" section

Append after the "Manual QA" section in `docs/TESTING.md`:

```markdown
## Bundle Inspection

After `pnpm build`, open `dist/stats.html` to see the chunk treemap (sizes are gzipped). Vendor libs are split:

| Chunk | Contents |
|---|---|
| `index-*.js` (entry) | App shell, Sidebar, Welcome modal, Toast, stores, react |
| `vendor-three` | three.js + @react-three/{fiber,drei,postprocessing} (Graphic3D module only) |
| `vendor-fabric` | fabric.js (Graphic2D + Typography modules) |
| `vendor-monaco` | @monaco-editor/react (WebsiteBuilder module) |
| `vendor-motion` | framer-motion |
| `vendor-react` | react-dom + react-router-dom |
| `vendor-misc` | lucide-react, zustand, lodash, etc. |
| Per-module chunks | Each `src/pages/*.tsx` lazy-loaded on route entry |

Module pages load on first route entry (Suspense fallback shows briefly). Switching between previously-loaded modules is instant.
```

## Testing

- `ModuleLoadingFallback.test.tsx` — renders without crashing + aria-busy
- `App.test.tsx` — existing routing tests must adapt to `findByText` (lazy imports resolve asynchronously); the assertion structure is preserved
- E2E `navigation.spec.ts` — module switches still work; Suspense fallback may briefly flash but disappears before assertion timeout
- E2E `welcome.spec.ts` — unaffected (modal mounts above the router)
- E2E `typography.spec.ts` — full happy-path still passes (Typography page lazy-loads on route entry)
- Manual: `pnpm build` produces main chunk < 600 KB minified; `dist/stats.html` browseable

## Risks

| Risk | Mitigation |
|---|---|
| Existing App.test.tsx routing tests break (synchronous `getByText` no longer resolves) | Convert to `findByText` / `findByRole` per failing case |
| Suspense + framer-motion AnimatePresence interaction shows blank frame | Mount Suspense INSIDE `<motion.div>` if the outer placement causes flicker |
| `manualChunks` regex misclassifies a module | Inspect `dist/stats.html` after build; refine regex |
| `react-dom` ends up in vendor chunk causing shell-render delay | Verify entry chunk still contains react-dom-driven shell render in the visualizer; if not, exclude `react-dom` from `vendor-react` |
| Tauri WebView2 (Windows) doesn't support dynamic import correctly | macOS WKWebView is Safari-based, full ES module support; no Tauri-side issue |
| First module load on cold WKWebView session has noticeable Suspense flash | Acceptable — falls under the same UX bar as Wave 5's Toast progress: visible "this is happening" beats frozen UI |

## Acceptance Criteria

This sub-project lands when:
- `pnpm build` produces a main entry chunk < 600 KB minified (target ~150-200 KB)
- Separate chunks present for `vendor-three`, `vendor-fabric`, `vendor-monaco`, plus per-module chunks for each `src/pages/*.tsx`
- `dist/stats.html` exists and renders a treemap showing the new chunk shape
- All 5 modules switch correctly via sidebar + Cmd+1-5 (Suspense fallback briefly visible if module not yet loaded)
- App.test.tsx + all other unit tests pass (with appropriate `findBy*` adaptations)
- E2E suite passes (12 specs)
- `pnpm exec tsc --noEmit` + `pnpm biome check .` clean; backend `cargo test` unchanged
- CI 5/5 green
- `docs/TESTING.md` extended with "Bundle inspection" section

After landing, Phase 8 is closed (Sub-Projects 1+2+3 done). Distribution remains explicitly out — revisited only after first live-test of the app.
