# Phase 8 Sub-Project 3 (Performance) — Verification Report

**Date:** 2026-04-19
**Spec:** `docs/superpowers/specs/2026-04-19-phase-8-performance-design.md`
**Plan:** `docs/superpowers/plans/2026-04-19-phase-8-performance.md`

## Summary

Lazy-loading + manual chunk splits delivered. Bundle reshape:

**Before (commit `7fa3a76`, baseline before Sub-Project 3):**
- main `index-*.js`: 2,341 kB minified / 688 kB gzipped (Vite warning)
- 3 split chunks: html2canvas, jsPDF, purify

**After (final, post-final-review fix):**
- main `index-*.js`: 39.69 kB minified / 12.09 kB gzipped — **−98%**
- vendor-react: 229.84 kB / 73.50 kB gzipped (react + react-dom + scheduler + react-router-dom — shell critical path with entry)
- vendor-fabric: 293.26 kB / 89.45 kB gzipped (Graphic2D + Typography, lazy)
- vendor-three: 1,007.19 kB / 276.40 kB gzipped (Graphic3D-only, lazy)
- vendor-monaco: 11.42 kB / 4.35 kB gzipped (WebsiteBuilder wrapper; Monaco itself is CDN, lazy)
- vendor-motion: 32.22 kB / 11.14 kB gzipped (framer-motion, eager — used by AnimatePresence/Toast/Tooltip)
- vendor-misc: 1,002.46 kB / 305.28 kB gzipped (lucide-react, zustand, gif.js, jspdf, html2canvas, etc., lazy in their consumer routes)
- 6 per-page chunks: 12.53 kB (WebsiteBuilder) – 25.27 kB (Graphic2D), gzipped 3.75–7.84 kB

**Shell critical path on first paint:** `index` (39.7 KB) + `vendor-react` (229.8 KB) + `vendor-motion` (32.2 KB) ≈ **302 KB minified / 96 KB gzipped** vs. baseline 2.34 MB / 688 KB gzipped — **−86% on shell-critical bytes** (the headline −98% number is the entry chunk alone, which is misleading because it doesn't include react-dom).

Vite's "chunks larger than 500 kB" warning persists for vendor-three + vendor-misc (intentional — they load on-demand for their owning routes, not on initial paint).

A circular-chunk warning (`vendor-three -> vendor-misc -> vendor-three`) surfaced after the react-dom move — likely a `@react-three/drei` dep that pulls something currently in vendor-misc back into the three boundary. Logged as backlog.

Verification pipeline:
- Frontend: `pnpm test` 382 tests / 78 files, all green
- Frontend coverage: 60.91% statements / 54.98% branches / 67.63% functions / 61.82% lines
- Backend: `cargo test` 432 tests passing / 4 ignored (unchanged — frontend-only sub-project)
- E2E: `pnpm e2e` 12 tests across 7 spec files (CI mode: 11 passed, 1 flaky-but-recovered on retry — pre-existing flake on `Mod+1-5` keyboard shortcut, reproduced on baseline `7fa3a76` so unrelated to lazy-loading)
- Lint: `pnpm biome check .` clean (210 files); `pnpm exec tsc --noEmit` clean
- CI: latest run green across all 5 jobs

## Pillar coverage

### 1. Lazy-loading
- 6 page components (`WebsiteBuilder`, `Graphic2D`, `Graphic3D`, `Video`, `Typography`, `DesignSystem`) wrapped in `React.lazy(() => import(...).then(m => ({ default: m.X })))`
- `<Suspense fallback={<ModuleLoadingFallback />}>` mounted inside `<AnimatePresence>`'s motion.div so route transitions still animate
- `ModuleLoadingFallback` mirrors the module shell (header tag + brief-row inputs + content area) via existing `<Skeleton />` primitive
- 6 App routing tests adapted from `getBy*` to `findBy*` (lazy resolution is async)

### 2. Vendor splits + visualizer
- `vite.config.ts` adds `build.rollupOptions.output.manualChunks` with regex matchers for three, fabric, monaco, framer-motion, and `react|react-dom|scheduler|react-router|react-router-dom` (the React core in `vendor-react`); everything else lands in `vendor-misc`
- React core (react + react-dom + scheduler) extracted into `vendor-react` after the final-review fix — without this, react-dom landed silently in `vendor-misc` and the shell critical path was ~1.2 MB instead of ~270 KB
- `rollup-plugin-visualizer@7.0.1` emits `dist/stats.html` (treemap, gzip sizes) on every build
- `docs/TESTING.md` documents chunks + how to inspect

## TODO ledger from final review
- Circular chunk `vendor-three ↔ vendor-misc` (introduced when react-dom moved to vendor-react). Logged as backlog. Likely a `@react-three/drei` transitive dep that the regex doesn't catch.
- `vendor-misc` still 1 MB / 305 KB gz — further splits possible (vendor-pdf for jspdf+html2canvas, vendor-icons for lucide-react). Backlog.
- No Playwright assertion that `ModuleLoadingFallback` actually renders during cold route entry. Backlog.

## Backlog filed during execution

none

## Phase 8 Closure

With Sub-Project 3 done, **all three Phase 8 sub-projects (Testing → UX-Polish → Performance) are closed**. Distribution (Code-Signing, DMG, Auto-Update, Landing Page) remains explicitly out of Phase 8 — revisited only after first live-test of the app.

**Phase 8 closed. Ready for live-test → Distribution → v1.0 release.**
