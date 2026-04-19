# Testing — Project Conventions

## Suites

| Suite | Command (local) | What it covers |
|---|---|---|
| Frontend unit + component | `pnpm test` | React components, stores, IPC wrapper modules |
| Frontend coverage | `pnpm test:coverage` | Same as above + lcov + HTML report under `coverage/` |
| Backend unit + integration | `cd src-tauri && cargo test` | Rust modules, pipelines, API-client wire-tests, integration suites under `src-tauri/tests/` |
| Backend coverage | `cd src-tauri && cargo llvm-cov --workspace --lcov --output-path ../coverage/backend.lcov` | Per-line/region/function coverage of the Rust workspace |
| End-to-end | `pnpm exec playwright test` | Module-flow happy paths; uses Approach A (browser-only with mocked Tauri `invoke`) |

## Coverage Target

**Aim for ≥80% line coverage on critical paths.** Critical paths are:

- All `#[tauri::command]` handlers in `src-tauri/src/`
- All `*Pipeline`, `*Generator`, `*Analyzer`, `*Builder` modules in `src-tauri/src/`
- All `src/pages/*.tsx` orchestrators
- All `src/lib/*Commands.ts` IPC wrappers

Coverage is **soft-gated**: CI uploads lcov as an artifact and prints a summary line, but no PR is blocked on coverage percentage. The metric is for discovery, not enforcement — it's easy to game by writing shallow tests, so a green PR with low coverage is a flag for the reviewer, not for CI.

## Wire-Tests for API Clients

Each of the 9 API clients in `src-tauri/src/api_clients/` has inline `#[cfg(test)]` tests using the `wiremock` crate. Coverage matrix (audit kept at `src-tauri/tests/api_clients_wire_audit.md`):

- Success-response parse
- Auth-header correctness
- Error-status mapping (4xx/5xx → typed error variants)
- Timeout behavior

When adding a new API client, replicate this matrix.

## E2E (Playwright)

Specs live under `e2e/tests/`. The `e2e/fixtures/invoke-mock.ts` fixture patches `window.__TAURI_INTERNALS__.invoke` so specs run against the Vite dev server without a real Tauri runtime — fast, deterministic, but does NOT exercise the Rust IPC layer (that's covered by `src-tauri/tests/*_integration.rs`). A future Sub-Project may add Tauri-WebDriver E2E (Approach C) for end-to-end IPC validation.

## CI

`.github/workflows/ci.yml` runs five jobs: `lint`, `test`, `coverage`, `e2e`, `build`. Coverage and e2e upload artifacts (lcov, Playwright trace) for inspection.

## Local Iteration

```bash
# Watch mode — frontend
pnpm test:watch

# Frontend coverage
pnpm test:coverage && open coverage/index.html

# Single Rust integration test
cd src-tauri && cargo test --test brand_kit_integration -- brand_kit_produces

# Single Playwright spec
pnpm exec playwright test e2e/tests/typography.spec.ts --headed
```

## Manual QA

Before tagging a release, walk `docs/QA-CHECKLIST.md` once. Update the checklist when you add module-level features.

## Bundle Inspection

After `pnpm build`, open `dist/stats.html` to see the chunk treemap (gzipped sizes). The build is split into:

| Chunk | Contents |
|---|---|
| `index-*.js` (entry) | App shell, Sidebar, Welcome modal, Toast, stores, react + react-dom |
| `vendor-three-*.js` | three.js + @react-three/{fiber,drei,postprocessing} (Graphic3D module only) |
| `vendor-fabric-*.js` | fabric.js (Graphic2D + Typography modules) |
| `vendor-monaco-*.js` | @monaco-editor/react wrapper (WebsiteBuilder module — Monaco itself is CDN-loaded by the wrapper) |
| `vendor-motion-*.js` | framer-motion (AnimatePresence, Toast, Tooltip animations) |
| `vendor-react-*.js` | react-router-dom (route-driven navigation) |
| `vendor-misc-*.js` | lucide-react, zustand, gif.js, jspdf, html2canvas, etc. |
| Per-page chunks | Each `src/pages/*.tsx` lazy-loaded on first route entry |

Module pages load on demand the first time their route is entered (a brief `<ModuleLoadingFallback />` skeleton appears). Switching back to a previously-loaded module is instant.

If the main entry chunk grows past ~200 KB minified, inspect `dist/stats.html` to find the new heavy import — usually a vendor lib that needs adding to `vite.config.ts`'s `manualChunks`.

Vite emits a "chunks larger than 500 kB" warning for `vendor-three` (~1 MB) and `vendor-misc` (~1.2 MB). This is intentional and acceptable — those chunks load on-demand for their owning routes, not on initial paint.
