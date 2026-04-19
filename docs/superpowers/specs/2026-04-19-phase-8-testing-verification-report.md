# Phase 8 Sub-Project 1 (Testing) — Verification Report

**Date:** 2026-04-19
**Spec:** `docs/superpowers/specs/2026-04-19-phase-8-testing-design.md`
**Plan:** `docs/superpowers/plans/2026-04-19-phase-8-testing.md`

## Summary

All 4 testing pillars implemented. Verification pipeline runs green:

- Frontend: `pnpm test` 345 tests / 69 files; `pnpm test:coverage` 60.12% lines / 54.30% branches
- Backend: `cargo test` 432 tests passing / 4 ignored; `cargo llvm-cov` 87.25% lines (87.61% region, 76.39% function)
- E2E: `pnpm e2e` 9 tests across 6 spec files
- Lint: `pnpm biome check .` clean (190 files); `pnpm exec tsc --noEmit` clean
- CI: latest run green — all 5 jobs (Lint / Test / Coverage / E2E / Build)

## Pillar coverage

### 1. Coverage Reporting
- **Frontend:** vitest v8-provider configured in `vite.config.ts`. Run via `pnpm test:coverage`. Reports at `coverage/lcov.info` + `coverage/index.html`.
- **Backend:** `cargo-llvm-cov` 0.6.18 produces `coverage/backend.lcov`.
- **CI artifact:** `coverage-reports` from the `coverage` job in `.github/workflows/ci.yml`.
- **Soft-gate:** documented in `docs/TESTING.md` — target ≥80% on critical paths, no PR-blocking threshold.

### 2. API-Client Wire-Tests
- Audit: `src-tauri/tests/api_clients_wire_audit.md` — all 9 clients covered across all 4 pillars (Success / Auth / Error / Timeout).
- Gaps closed: 5 timeout-pillar tests added (claude/kling/ideogram/fal/replicate) in commit `5ad3a1d`.
- Final coverage matrix: 36 ✓ / 0 ✗.

### 3. Playwright E2E (Approach A)
- 6 spec files / 9 tests under `e2e/tests/`:
  - `navigation.spec.ts` (4)
  - `typography.spec.ts` (1, full Phase 7 happy-path)
  - `website-builder.spec.ts` (1, smoke)
  - `graphic2d.spec.ts` (1, smoke)
  - `graphic3d.spec.ts` (1, smoke)
  - `video.spec.ts` (1, smoke)
- Fixture: `e2e/fixtures/invoke-mock.ts` patches `window.__TAURI_INTERNALS__.invoke` per spec.
- CI: `e2e` job in `.github/workflows/ci.yml`, chromium-cached, 43s cold first run.

### 4. Manual QA Checklist
- `docs/QA-CHECKLIST.md` — 42 bullets across 7 sections (Boot 3 + Website Builder 6 + Graphic 2D 6 + Graphic 3D 5 + Video 5 + Typography 11 + Cross-cutting 6). Content is verbatim from plan T15.

## Backlog touched
- **#177 (POST-PHASE-7):** Integration test ABI coverage — the Playwright invoke-mock fixture provides a substrate to revisit; the backlog item stays open until someone ports the original Typography integration test into the Playwright suite OR upgrades the mock to assert command name + payload at the invoke boundary.

## Backlog filed during execution
- **#197:** `for_test_with_http_timeout` constructor for sync API clients (deferred optimization to drop ~5s wall-clock from timeout tests when 6+ sync clients exist).
- **#198:** Mini-fix in `docs/TESTING.md` — references "four jobs" in the CI section but five exist post-coverage; one-line update.

## Verdict

Phase 8 Sub-Project 1 (Testing) closed. Coverage is now visible (CI reports), wire-test gaps are closed, E2E covers module flows, and the manual QA checklist captures what tests can't.

**Ready to brainstorm Sub-Project 2 (UX-Polish).**
