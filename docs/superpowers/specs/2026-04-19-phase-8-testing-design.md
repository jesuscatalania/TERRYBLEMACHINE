# Phase 8 — Sub-Project 1: Testing (Design)

**Date:** 2026-04-19
**Phase context:** Phase 8 ("Polish & Release") was decomposed into three sub-projects executed in this order: **Testing → UX-Polish → Performance**. Distribution (8.4) is dropped from Phase 8 entirely and revisited only after the first live-test of the app.
**Source plan:** `docs/ENTWICKLUNGSPLAN.md` lines 355-388 (Phase 8.1-8.3)

## Goal

Close the project's testing-coverage gaps before any further refactor work in UX-Polish or Performance. Specifically:

1. Make existing test coverage **visible** (today: ~342 frontend Vitest tests, 425+ Rust tests, no coverage reporting in CI)
2. Add **API-client wire-tests** with mocked HTTP servers (today: HTTP-layer parsing/auth/error-mapping is implicitly covered through pipeline mocks but never asserted against real wire bytes)
3. Add **end-to-end UI tests** via Playwright (today: zero E2E tests exist)
4. Capture **manual-QA edge cases** as a written checklist (today: lives only in the developer's head)

## Non-Goals

- **Distribution (Code-Signing, DMG, Auto-Update, Landing Page):** Out of Phase 8 entirely; revisited after first live-test.
- **Tauri-WebDriver E2E (Approach C):** A real-IPC E2E layer using `tauri-driver`; deferred to a post-Phase-8 backlog item. Browser-only E2E (Approach A) is sufficient to close the immediate frontend-flow gap.
- **Visual regression tests:** Out of scope.
- **A11y audit (axe-core):** Out of scope.
- **Performance benchmarks/profiling tests:** Out of scope here; lives in Sub-Project 3 (Performance).
- **Hard coverage gates in CI:** Reports are uploaded as artifacts; threshold (≥80% on critical paths) is documented as a target but not enforced as a PR-blocker (avoids gaming the metric on legitimate test gaps).

## Architecture — Where Things Live

| Component | Location |
|---|---|
| Frontend coverage | `vitest.config.ts` extension — v8 provider via `pnpm vitest run --coverage` |
| Backend coverage | `cargo-llvm-cov` as a dev-tool, lcov output |
| API-client wire-tests | `src-tauri/tests/api_clients_wire/{claude,kling,runway,higgsfield,shotstack,ideogram,meshy,fal,replicate}.rs` using the `wiremock` crate |
| Playwright E2E | new top-level `e2e/` directory: `playwright.config.ts`, `fixtures/invoke-mock.ts`, `tests/*.spec.ts` |
| Manual-QA checklist | `docs/QA-CHECKLIST.md` |
| CI | `.github/workflows/ci.yml` — new jobs: `coverage` (frontend + backend), `e2e` (Playwright). Coverage as **soft-gate**: artifact upload + summary, no PR-blocking threshold |

## Components in Detail

### 1. Coverage Reporting

**Frontend:**
- Add coverage config to `vitest.config.ts`: `coverage: { provider: "v8", reporter: ["text", "lcov", "html"], reportsDirectory: "./coverage/frontend" }`
- New script in `package.json`: `"test:coverage": "vitest run --coverage"`
- CI job uploads `coverage/frontend/lcov.info` + html report as artifacts

**Backend:**
- Add `cargo-llvm-cov` to README's "dev tools" section (installed via `cargo install cargo-llvm-cov` or via rustup component)
- New CI step: `cargo llvm-cov --workspace --lcov --output-path coverage/backend/lcov.info`
- Backend coverage may underreport on `#[tauri::command]` functions (they're called via macro-generated wrappers); document this caveat in a comment near the script

**Soft-gate semantics:**
- CI emits a markdown summary line: `Frontend coverage: X% lines / Y% branches; Backend: Z% lines`
- A `docs/TESTING.md` documents the **target ≥80% on critical paths** (critical paths defined as: all Tauri command handlers, all `*Pipeline` modules in `src-tauri/src/`, all `pages/*.tsx` orchestrators, all `lib/*Commands.ts` wrappers)
- No PR is blocked by coverage; the metric is a discovery tool, not a gate

### 2. API-Client Wire-Tests

**Purpose:** Today the 9 API clients (`src-tauri/src/api_clients/{claude,kling,runway,higgsfield,shotstack,ideogram,meshy,fal,replicate}.rs`) are tested through `AiRouter` unit tests with trait-mocks. That covers the *trait contract* but NOT the HTTP layer (JSON-body parsing, auth-header construction, content-type handling, error-status mapping). A provider that silently changes their response schema would not be caught.

**Approach:** `wiremock` Rust crate stands up a real HTTP server bound to a localhost port; the API client is pointed at that server via a configurable base URL. Each test asserts:

1. **Success-response parse:** server returns canonical happy-path JSON; client returns the expected typed result
2. **Auth-header correctness:** server's request matcher asserts the expected header (e.g., `Authorization: Bearer <key>`)
3. **Error-status mapping:** server returns 401/429/500; client returns the correct error variant
4. **Timeout behavior:** server delays past timeout; client returns a Timeout error

**Setup:** API clients need to accept a `base_url` parameter (most do already for testing; verify per-client and add where missing). The `KeyStore` dependency gets an `InMemoryKeyStore` test-double.

**File layout:**
```
src-tauri/tests/api_clients_wire/
├── common.rs        # InMemoryKeyStore + base setup helpers
├── claude.rs
├── kling.rs
├── runway.rs
├── higgsfield.rs
├── shotstack.rs
├── ideogram.rs
├── meshy.rs
├── fal.rs
└── replicate.rs
```

(Cargo treats each file in `tests/api_clients_wire/` as a separate crate by default. To share `common.rs`, the directory needs a `mod.rs`-style binary or each file uses `#[path = "common.rs"] mod common;`. Decision: use `#[path]` per-file to keep the crate boundaries simple.)

**Per-file structure:** ~3-4 tests per client × 9 clients = ~30 new integration tests.

**`wiremock` is added as a dev-dependency only** — zero runtime impact, no API ABI changes.

### 3. Playwright E2E (Approach A: browser-only with invoke-mock)

**Why Approach A** (recap from brainstorming):
- Real Tauri-IPC is already covered by Rust integration tests (one `*_integration.rs` per pipeline)
- The actual E2E gap is *frontend-flow behavior*: routing, store updates, multi-step user flows
- `tauri-driver` on macOS uses `safaridriver` which has reliability quirks; would introduce CI flakiness
- Approach C (Tauri-WebDriver) is deferred to a post-Phase-8 backlog item

**Stack:**
- `@playwright/test` as dev-dependency
- New top-level `e2e/` directory (sibling to `src/`, `src-tauri/`)
- `e2e/playwright.config.ts` — `webServer: { command: 'pnpm dev', url: 'http://localhost:1420' }`, `baseURL: 'http://localhost:1420'`, headless
- `e2e/fixtures/invoke-mock.ts` — page-script injected before `goto`. Patches `window.__TAURI_INTERNALS__.invoke` (Tauri v2 internal; verify exact path against current Tauri version) with a configurable response-map. Each test sets up which command names → which response payloads.

**Specs (one per module + cross-module navigation):**

| Spec | Flow exercised |
|---|---|
| `website-builder.spec.ts` | enter prompt → click Generate → mocked `generate_website` returns HTML → preview pane shows it |
| `graphic2d.spec.ts` | enter prompt → click Generate → mocked `text_to_image` returns 6 variants → click variant → editor shows |
| `graphic3d.spec.ts` | upload image → mocked `generate_depth` returns map → mocked `generate_mesh_from_image` returns gltf path |
| `video.spec.ts` | enter prompt → mocked `generate_storyboard` returns 4 shots → mocked `assemble_video` returns mp4 path → preview |
| `typography.spec.ts` | full Phase-7 happy-path: generate → vectorize → addText → export brand kit (mocked all four commands) |
| `navigation.spec.ts` | sidebar module switches preserve in-flight state; deep-link `/typography` lands directly; browser-back works |

**CI job:**
- Installs Playwright browsers via `pnpm exec playwright install --with-deps chromium` (cached)
- Runs `pnpm exec playwright test`
- On failure: uploads `playwright-report/` as CI artifact (HTML report + traces)

### 4. Manual QA Checklist

**File:** `docs/QA-CHECKLIST.md`

**Format:** Markdown checkboxes, one section per module:

```markdown
## Typography Module
- [ ] First-use: empty state shows "No logos yet"
- [ ] Generate happy path: prompt + style + palette → 6 variants
- [ ] Generate empty prompt: button disabled
- [ ] Generate API-key missing: error toast surfaces
- [ ] Vectorize: variant with local_path → SVG renders in editor
- [ ] Vectorize without local_path: button disabled, hint shown
- [ ] Add text: empty input disables button
- [ ] Export: missing source PNG → error toast
- [ ] Export: success → toast with zip path
- [ ] Favorites filter: heart toggles, "Show favorites only" filters, resets on regenerate
```

**Modules:** Website / Graphic2D / Graphic3D / Video / Typography / Design System (~10 bullets each).

**Usage:** Developer (or future tester) runs through the checklist before tagging a release. Lives in repo so it evolves with the code.

## Data Flow

- **Local dev:** `pnpm test:coverage` + `cargo llvm-cov` + `pnpm exec playwright test` are independently runnable
- **CI:** parallel jobs (existing Vitest + new coverage + new e2e + existing cargo test). Coverage uploads as artifact; Playwright traces upload only on failure.
- **Mock-server lifecycle:** each `wiremock` test starts a fresh `MockServer` (random port), tears it down at end of test; no shared state.
- **Playwright invoke-mock:** initialized via `page.addInitScript()` BEFORE the first `page.goto()` so the React app sees the patched `__TAURI_INTERNALS__` from first render.

## Error Handling

- All test suites must exit 0 for CI green.
- Coverage thresholds: warning-only summary, never failing the build.
- Playwright failures upload trace artifact; the failing spec name is in the CI summary.
- `wiremock` test that fails to start its mock server (port collision) returns the standard `tokio` error; no special handling.

## Testing Approach

- **TDD where applicable:** Playwright invoke-mock fixture is itself a small piece of code with branching logic — has its own unit test under `e2e/fixtures/invoke-mock.test.ts` (using Vitest, not Playwright, since it's a TS module not a browser test).
- **No new tests for the API-client wire-tests themselves** — they ARE tests; meta-testing would not add value.
- **Coverage report parity:** the `coverage` CI job that runs `vitest --coverage` must produce the same test count as the existing `test` job; if numbers diverge, the coverage step is misconfigured.

## Risk Assessment

| Risk | Likelihood | Mitigation |
|---|---|---|
| `wiremock` adds significant compile time to dev workflow | Low | wiremock is a dev-dep only; `cargo test` cold-build adds ~10s, acceptable |
| Playwright Chromium download bloats CI | Medium | Cache Playwright browsers in CI; first run ~150MB, subsequent runs cached |
| `tauri-driver` quirks affect us anyway | None | We chose Approach A explicitly to avoid it |
| Soft-gate coverage gets ignored | Medium | Document target in `docs/TESTING.md`; review coverage trend in retros |
| Playwright invoke-mock drifts from real Tauri ABI | Medium | Backlog #177 covers ABI-coverage hardening; this design's tests still validate frontend flow correctness |
| 6 E2E specs grow stale if backend ABI changes | Low (today) — Medium (long-term) | Mock-response shapes are typed against the real `*Commands.ts` TS interfaces, so an interface drift fails the TS compile |

## Out of Scope (Explicitly)

- Tauri-WebDriver E2E (Approach C) → backlog item to be filed
- Performance-profiling tests → Sub-Project 3
- Visual-regression tests → backlog
- A11y / axe audit → backlog
- Distribution / signing / installer / auto-update → out of Phase 8 entirely

## Existing Backlog Item Touched

- **#177 (POST-PHASE-7 BACKLOG):** "Integration test ABI coverage for Typography flow" — was filed when Phase 7's `Typography.integration.test.tsx` mocked the *Commands wrappers rather than `invoke`. The fixture built in this Sub-Project (`e2e/fixtures/invoke-mock.ts`) provides the right substrate to revisit #177; the backlog item stays open and can be resolved by porting the integration test into the Playwright suite.

## Verdict / Acceptance Criteria

This sub-project lands when:
- `pnpm test:coverage` produces an HTML coverage report locally and lcov in CI
- `cargo llvm-cov` produces an lcov report in CI
- All 9 API clients have a wiremock-based wire-test suite (≥3 tests each)
- All 6 Playwright specs run green in CI against the dev server
- `docs/QA-CHECKLIST.md` exists with ≥10 bullets per module
- `docs/TESTING.md` documents the coverage target + critical-path definition
- CI is green; no test suite was loosened or skipped to make it pass

After landing, brainstorm Sub-Project 2 (UX-Polish).
