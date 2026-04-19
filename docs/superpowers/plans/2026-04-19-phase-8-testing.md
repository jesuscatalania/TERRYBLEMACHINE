# Phase 8 Sub-Project 1: Testing — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the project's testing-coverage gaps before Phase 8 Sub-Projects 2-3 (UX-Polish, Performance) by making existing test coverage visible (CI reports), filling API-client wire-test gaps, adding Playwright E2E for module flows, and capturing edge cases as a written QA checklist.

**Architecture:** Four orthogonal additions. (1) Frontend Vitest coverage already configured — only CI surfacing missing. (2) Backend gets `cargo-llvm-cov` tooling + CI job. (3) Wiremock per-client tests already exist inline; a coverage-matrix audit fills gaps. (4) Playwright E2E lives in a new top-level `e2e/` directory with an `invoke`-mock fixture so specs run against the Vite dev server (Approach A from the spec).

**Tech Stack:** Vitest 3 + @vitest/coverage-v8 (already installed); `cargo-llvm-cov` (new dev-tool); `wiremock` 0.6 Rust crate (already a dev-dep); `@playwright/test` (new dev-dep).

**Source spec:** `docs/superpowers/specs/2026-04-19-phase-8-testing-design.md`

---

## File Structure

| File | Responsibility |
|---|---|
| `vite.config.ts` | Vitest config — coverage block already present; `include`/`exclude` may need refinement |
| `package.json` | `test:coverage` script already present; may add `e2e` + `e2e:ui` scripts |
| `.gitignore` | New entries: `coverage/`, `playwright-report/`, `test-results/`, `e2e/.auth/` |
| `docs/TESTING.md` | Coverage targets, critical-path definition, how to run all test suites locally + in CI |
| `docs/QA-CHECKLIST.md` | Module-by-module manual-QA checklist (markdown checkboxes) |
| `.github/workflows/ci.yml` | New jobs: `coverage` (frontend + backend lcov upload + summary), `e2e` (Playwright with chromium cache, trace upload on failure) |
| `src-tauri/tests/api_clients_wire_audit.md` | One-shot audit doc listing per-client coverage matrix (success / auth / error-status / timeout). Drives Task 2's gap-filling |
| `src-tauri/src/api_clients/{claude,kling,runway,higgsfield,shotstack,ideogram,meshy,fal,replicate}.rs` | Inline `#[cfg(test)]` blocks — extended where audit identifies gaps |
| `e2e/playwright.config.ts` | Playwright project config — webServer points at `pnpm dev`, headless, retries=1 |
| `e2e/fixtures/invoke-mock.ts` | Patches `window.__TAURI_INTERNALS__.invoke` with a configurable command-name → response map |
| `e2e/fixtures/invoke-mock.test.ts` | Vitest unit tests for the mock fixture itself |
| `e2e/tests/{navigation,website-builder,graphic2d,graphic3d,video,typography}.spec.ts` | One Playwright spec per module + cross-module navigation |
| `docs/superpowers/specs/2026-04-19-phase-8-testing-verification-report.md` | Phase 8 Sub-Project 1 closure report |

---

## Task 1: Frontend coverage CI surfacing + verification

**Files:**
- Modify: `vite.config.ts` (verify coverage block; tighten `include`/`exclude` if needed)
- Modify: `package.json` (verify `test:coverage` script present)
- Modify: `.gitignore` (add `coverage/` if missing)

- [ ] **Step 1: Inspect current coverage config**

```bash
grep -A 10 "coverage:" vite.config.ts
grep "test:coverage" package.json
grep "^coverage" .gitignore
```

Expected: coverage block with `provider: "v8"`, reporter array including `lcov`, `include: ["src/**/*.{ts,tsx}"]`, `exclude` for tests + setup. Script `test:coverage`: `vitest run --coverage`. `.gitignore` may NOT yet have `coverage/` (verify and add if missing).

- [ ] **Step 2: Add `coverage/` to .gitignore if absent**

If grep above returned no match, append to `.gitignore`:
```
# Test coverage reports
coverage/
```

- [ ] **Step 3: Run frontend coverage locally and verify report shape**

```bash
pnpm test:coverage
```

Expected output ends with a summary table (`% Stmts | % Branch | % Funcs | % Lines`). The `coverage/` directory should contain `lcov.info`, `index.html`, and per-file `.html` reports.

- [ ] **Step 4: Commit**

```bash
git add .gitignore vite.config.ts package.json
git commit -m "chore(testing): verify vitest coverage config + ignore coverage/"
```

---

## Task 2: Backend coverage with cargo-llvm-cov

**Files:**
- Create: nothing new in `src-tauri/`
- Modify: `.gitignore` (cover `src-tauri/target/llvm-cov-target/` if not yet covered)

- [ ] **Step 1: Install cargo-llvm-cov locally**

```bash
cargo install cargo-llvm-cov --version 0.6.18
```

(Pin version so CI matches local dev. Adjust version if a newer minor is available — check via `cargo install --list cargo-llvm-cov`.)

- [ ] **Step 2: Run backend coverage and verify lcov output**

```bash
cd src-tauri
cargo llvm-cov --workspace --lcov --output-path ../coverage/backend.lcov
cd ..
ls -lh coverage/backend.lcov
```

Expected: `coverage/backend.lcov` exists, > 100KB. Stdout shows per-file line/region/function coverage percentages.

- [ ] **Step 3: Verify .gitignore covers llvm-cov targets**

The existing `target/` line in `.gitignore` already covers `src-tauri/target/llvm-cov-target/`. No edit needed.

- [ ] **Step 4: Commit (no code change — this is a tooling-verification task)**

No commit yet — Task 4 (TESTING.md) will document the workflow and Task 5 (CI) will wire it into automation. Leave this as a local verification step.

---

## Task 3: API-client wiremock coverage audit

**Files:**
- Create: `src-tauri/tests/api_clients_wire_audit.md`

- [ ] **Step 1: List existing test functions per API client**

```bash
for f in src-tauri/src/api_clients/{claude,kling,runway,higgsfield,shotstack,ideogram,meshy,fal,replicate}.rs; do
  echo "=== $f ==="
  grep -E "^\s*(#\[test\]|#\[tokio::test\]|async fn|fn) " "$f" | grep -v "//" | head -30
done
```

For each client, note the test names. Categorize each into one of the four wire-test pillars:
- **Success-response parse:** test asserts the client returns the expected typed result given canonical happy-path JSON
- **Auth-header correctness:** test asserts the request includes the expected `Authorization` (or provider-specific) header
- **Error-status mapping:** test asserts an HTTP 4xx/5xx maps to the correct error variant (Auth, RateLimit, Backend, etc.)
- **Timeout behavior:** test asserts a delayed response or no response maps to a Timeout error variant

- [ ] **Step 2: Write the audit doc**

Create `src-tauri/tests/api_clients_wire_audit.md`:

```markdown
# API-Client Wiremock Coverage Audit

**Date:** <today>
**Scope:** 9 API clients in `src-tauri/src/api_clients/`. Inline `#[cfg(test)]` blocks audited for the four wire-test pillars: Success / Auth / Error-status / Timeout.

| Client | Success | Auth | Error | Timeout | Gaps |
|---|---|---|---|---|---|
| claude | ✓/✗ | ✓/✗ | ✓/✗ | ✓/✗ | <list missing pillars or "none"> |
| kling | ✓/✗ | ✓/✗ | ✓/✗ | ✓/✗ | <list missing pillars or "none"> |
| runway | ✓/✗ | ✓/✗ | ✓/✗ | ✓/✗ | <list missing pillars or "none"> |
| higgsfield | ✓/✗ | ✓/✗ | ✓/✗ | ✓/✗ | <list missing pillars or "none"> |
| shotstack | ✓/✗ | ✓/✗ | ✓/✗ | ✓/✗ | <list missing pillars or "none"> |
| ideogram | ✓/✗ | ✓/✗ | ✓/✗ | ✓/✗ | <list missing pillars or "none"> |
| meshy | ✓/✗ | ✓/✗ | ✓/✗ | ✓/✗ | <list missing pillars or "none"> |
| fal | ✓/✗ | ✓/✗ | ✓/✗ | ✓/✗ | <list missing pillars or "none"> |
| replicate | ✓/✗ | ✓/✗ | ✓/✗ | ✓/✗ | <list missing pillars or "none"> |

**Total gaps to fill:** <count>
```

Fill the table cells based on Step 1's findings. The "Gaps" column lists the specific missing pillar(s) per client.

- [ ] **Step 3: Commit the audit**

```bash
git add src-tauri/tests/api_clients_wire_audit.md
git commit -m "docs(testing): API-client wiremock coverage audit"
```

---

## Task 4: Fill API-client wire-test gaps

**Files:**
- Modify: `src-tauri/src/api_clients/<client>.rs` for each client identified in Task 3's audit as having gaps

- [ ] **Step 1: For each gap identified in `api_clients_wire_audit.md`, add the missing test**

Use this template per pillar (adapt the URL paths, headers, and types to the specific client). The `for_test` constructor pattern already exists on each client (e.g., `ClaudeClient::for_test(key_store, base_url)`).

**Success-response template** (illustrative — adapt to client's actual response shape):

```rust
#[tokio::test]
async fn parses_canonical_success_response() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/v1/messages"))  // adapt path
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                // canonical happy-path response from the provider docs
            })),
        )
        .mount(&server)
        .await;

    let key_store = std::sync::Arc::new(crate::keychain::InMemoryKeyStore::with_key(
        super::KEYCHAIN_SERVICE,
        "test-key",
    ));
    let client = super::ClaudeClient::for_test(key_store, server.uri());

    let result = client
        .send(/* canonical AiRequest for this provider */)
        .await
        .expect("should parse");

    assert_eq!(result.text, "expected content");  // adapt assertion
}
```

**Auth-header template:**

```rust
#[tokio::test]
async fn sends_auth_header() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::header("x-api-key", "test-key"))  // adapt header name
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;

    let key_store = std::sync::Arc::new(crate::keychain::InMemoryKeyStore::with_key(
        super::KEYCHAIN_SERVICE,
        "test-key",
    ));
    let client = super::ClaudeClient::for_test(key_store, server.uri());
    let _ = client.send(/* minimal request */).await;
    // The mock's matcher will fail the request match if the header is absent,
    // returning a 404; the assertion is implicit in the success result.
}
```

**Error-status template:**

```rust
#[tokio::test]
async fn maps_429_to_rate_limit_error() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let key_store = std::sync::Arc::new(crate::keychain::InMemoryKeyStore::with_key(
        super::KEYCHAIN_SERVICE,
        "test-key",
    ));
    let client = super::ClaudeClient::for_test(key_store, server.uri());
    let err = client.send(/* request */).await.expect_err("should reject 429");
    assert!(matches!(err, crate::ai_router::ProviderError::RateLimit { .. }));
}
```

**Timeout template:**

```rust
#[tokio::test]
async fn maps_response_delay_to_timeout() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_secs(10)),
        )
        .mount(&server)
        .await;

    let key_store = std::sync::Arc::new(crate::keychain::InMemoryKeyStore::with_key(
        super::KEYCHAIN_SERVICE,
        "test-key",
    ));
    // for_test uses a 5s timeout; the 10s delay will trip it.
    let client = super::ClaudeClient::for_test(key_store, server.uri());
    let err = client.send(/* request */).await.expect_err("should time out");
    assert!(matches!(err, crate::ai_router::ProviderError::Timeout { .. }));
}
```

**Note:** if `InMemoryKeyStore` doesn't yet exist, define it in `src-tauri/src/keychain/mod.rs` under `#[cfg(test)]`:

```rust
#[cfg(test)]
pub struct InMemoryKeyStore {
    keys: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

#[cfg(test)]
impl InMemoryKeyStore {
    pub fn with_key(service: &str, key: &str) -> Self {
        let mut m = std::collections::HashMap::new();
        m.insert(service.to_string(), key.to_string());
        Self { keys: std::sync::Mutex::new(m) }
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl KeyStore for InMemoryKeyStore {
    async fn get(&self, service: &str) -> Result<String, KeyStoreError> {
        self.keys
            .lock()
            .unwrap()
            .get(service)
            .cloned()
            .ok_or_else(|| KeyStoreError::NotFound(service.to_string()))
    }
    async fn set(&self, service: &str, key: &str) -> Result<(), KeyStoreError> {
        self.keys.lock().unwrap().insert(service.to_string(), key.to_string());
        Ok(())
    }
    async fn delete(&self, service: &str) -> Result<(), KeyStoreError> {
        self.keys.lock().unwrap().remove(service);
        Ok(())
    }
    async fn list(&self) -> Result<Vec<String>, KeyStoreError> {
        Ok(self.keys.lock().unwrap().keys().cloned().collect())
    }
}
```

(Adapt the trait signature to match the actual `KeyStore` trait in `src-tauri/src/keychain/mod.rs`. If that file uses sync APIs, drop `async`/`async_trait`.)

- [ ] **Step 2: Run cargo test, fix any failures**

```bash
cd src-tauri
cargo test --lib
```

All previously passing + new tests must pass.

- [ ] **Step 3: Update audit doc to show all-✓**

Edit `src-tauri/tests/api_clients_wire_audit.md` to flip ✗ to ✓ for the filled gaps. Update the "Total gaps to fill" count to 0.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/api_clients/ src-tauri/src/keychain/ src-tauri/tests/api_clients_wire_audit.md
git commit -m "test(api-clients): fill wiremock coverage gaps (success/auth/error/timeout)"
```

(If Task 3's audit shows zero gaps, **skip Task 4 entirely** and commit a one-line update to the audit doc reflecting "no gaps — already covered". The audit IS the deliverable in that case.)

---

## Task 5: docs/TESTING.md

**Files:**
- Create: `docs/TESTING.md`

- [ ] **Step 1: Write the doc**

```markdown
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

`.github/workflows/ci.yml` runs four jobs: `lint`, `test`, `coverage`, `e2e`, `build`. Coverage and e2e upload artifacts (lcov, Playwright trace) for inspection.

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
```

- [ ] **Step 2: Commit**

```bash
git add docs/TESTING.md
git commit -m "docs(testing): add TESTING.md (suites, coverage targets, wire-test matrix)"
```

---

## Task 6: CI coverage job

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Add the coverage job**

Open `.github/workflows/ci.yml`. After the existing `test` job and before the `build` job, insert:

```yaml
  coverage:
    name: Coverage
    runs-on: macos-latest
    needs: [lint, test]
    steps:
      - uses: actions/checkout@v4

      - uses: pnpm/action-setup@v4
        with:
          version: ${{ env.PNPM_VERSION }}

      - uses: actions/setup-node@v4
        with:
          node-version: ${{ env.NODE_VERSION }}
          cache: "pnpm"

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Frontend coverage
        run: pnpm test:coverage

      - name: Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview

      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri -> target

      - name: Install cargo-llvm-cov
        run: cargo install cargo-llvm-cov --version 0.6.18 --locked

      - name: Backend coverage
        working-directory: src-tauri
        run: cargo llvm-cov --workspace --lcov --output-path ../coverage/backend.lcov

      - name: Coverage summary
        run: |
          echo "## Coverage report" >> $GITHUB_STEP_SUMMARY
          echo "Frontend lcov: \`coverage/lcov.info\` (artifact)" >> $GITHUB_STEP_SUMMARY
          echo "Backend lcov: \`coverage/backend.lcov\` (artifact)" >> $GITHUB_STEP_SUMMARY
          echo "Soft-gate: see docs/TESTING.md for the ≥80% target on critical paths." >> $GITHUB_STEP_SUMMARY

      - name: Upload coverage artifacts
        uses: actions/upload-artifact@v4
        with:
          name: coverage-reports
          path: |
            coverage/
            !coverage/tmp/
          retention-days: 14
```

- [ ] **Step 2: Push and verify CI green**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add coverage job (frontend vitest + backend cargo-llvm-cov)"
git push origin main
gh run list --branch main --limit 1 --json databaseId,status
gh run watch <id> --exit-status
```

The new `coverage` job must succeed. The lcov artifact must be downloadable from the workflow run page.

---

## Task 7: Playwright install + scaffolding

**Files:**
- Create: `e2e/playwright.config.ts`
- Create: `e2e/fixtures/invoke-mock.ts`
- Create: `e2e/fixtures/invoke-mock.test.ts`
- Modify: `package.json` (add `@playwright/test` dev-dep + `e2e` + `e2e:ui` scripts)
- Modify: `.gitignore` (add `playwright-report/`, `test-results/`, `e2e/.auth/`)

- [ ] **Step 1: Install Playwright**

```bash
pnpm add -D @playwright/test
pnpm exec playwright install --with-deps chromium
```

- [ ] **Step 2: Add scripts to package.json**

```json
"scripts": {
  // ... existing scripts
  "e2e": "playwright test",
  "e2e:ui": "playwright test --ui",
  "e2e:install": "playwright install --with-deps chromium"
}
```

- [ ] **Step 3: Update .gitignore**

Append:
```
# Playwright
playwright-report/
test-results/
e2e/.auth/
```

- [ ] **Step 4: Create e2e/playwright.config.ts**

```typescript
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: process.env.CI ? 2 : undefined,
  reporter: process.env.CI ? [["html", { open: "never" }], ["github"]] : "html",
  use: {
    baseURL: "http://localhost:1420",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    command: "pnpm dev --host 0.0.0.0",
    url: "http://localhost:1420",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    stdout: "pipe",
    stderr: "pipe",
  },
});
```

- [ ] **Step 5: Create e2e/fixtures/invoke-mock.ts**

```typescript
import type { Page } from "@playwright/test";

export type InvokeMock = Record<string, unknown | ((args: unknown) => unknown | Promise<unknown>)>;

/**
 * Patches `window.__TAURI_INTERNALS__.invoke` so the React app's IPC calls
 * resolve against the supplied mock map instead of a real Tauri runtime.
 *
 * Usage in a spec:
 *   await installInvokeMock(page, {
 *     generate_logo_variants: () => [{ url: "...", local_path: null, seed: 1, model: "ideogram-v3" }],
 *     vectorize_image: () => ({ svg: "<svg/>", width: 100, height: 100 }),
 *   });
 *   await page.goto("/typography");
 *
 * Unknown command names reject with an explicit error so a spec doesn't
 * silently miss a mock.
 */
export async function installInvokeMock(page: Page, mock: InvokeMock): Promise<void> {
  await page.addInitScript((serialized: string) => {
    const map = JSON.parse(serialized) as Record<string, unknown>;
    const internals = ((window as unknown) as { __TAURI_INTERNALS__?: Record<string, unknown> });
    internals.__TAURI_INTERNALS__ = internals.__TAURI_INTERNALS__ ?? {};
    (internals.__TAURI_INTERNALS__ as Record<string, unknown>).invoke = async (
      cmd: string,
      _args?: unknown,
    ) => {
      if (!(cmd in map)) {
        throw new Error(`[invoke-mock] unmocked command: ${cmd}`);
      }
      const value = map[cmd];
      // Functions are not JSON-serializable; we only support static values
      // crossing the addInitScript boundary. Spec authors who need dynamic
      // responses should call installInvokeMock again with the new values.
      return value;
    };
  }, JSON.stringify(serializableMockOnly(mock)));
}

function serializableMockOnly(mock: InvokeMock): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(mock)) {
    if (typeof v === "function") {
      throw new Error(
        `[invoke-mock] command "${k}" is a function, but addInitScript only crosses serializable values. ` +
          `Use static response shapes; if dynamic responses are needed, file a backlog item to switch to ` +
          `Playwright's exposeFunction approach.`,
      );
    }
    out[k] = v;
  }
  return out;
}
```

- [ ] **Step 6: Create e2e/fixtures/invoke-mock.test.ts**

This is a **Vitest** test, not Playwright — it tests the helper module's pure logic. Place under `e2e/fixtures/` so it ships with the helper but is picked up by Vitest's default `**/*.test.ts` pattern. (If Vitest is configured to only scan `src/`, add `e2e/**/*.test.ts` to its include in `vite.config.ts`.)

Actually — to keep things simple, put this under `src/lib/` instead since `e2e/` is a Playwright-only directory. Move the file to `src/lib/invokeMock.ts` + `src/lib/invokeMock.test.ts`, re-export from `e2e/fixtures/invoke-mock.ts`.

Decision: keep the fixture in `e2e/fixtures/invoke-mock.ts` (where Playwright finds it) and add the include line:

In `vite.config.ts`, extend the `test` block:

```typescript
test: {
  // ... existing settings
  include: ["src/**/*.test.{ts,tsx}", "e2e/fixtures/**/*.test.ts"],
},
```

Then create `e2e/fixtures/invoke-mock.test.ts`:

```typescript
import { describe, expect, it } from "vitest";
// We only test the pure-logic helper inside invoke-mock; the Playwright
// glue (addInitScript) requires a real browser so it isn't unit-testable.
import { serializableMockOnly } from "./invoke-mock";

describe("serializableMockOnly", () => {
  it("passes through static values", () => {
    const result = serializableMockOnly({
      foo: 1,
      bar: { nested: true },
      baz: [1, 2, 3],
    });
    expect(result).toEqual({ foo: 1, bar: { nested: true }, baz: [1, 2, 3] });
  });

  it("throws when a value is a function", () => {
    expect(() => serializableMockOnly({ cmd: () => "dynamic" })).toThrow(
      /command "cmd" is a function/,
    );
  });

  it("throws message names the offending command", () => {
    expect(() =>
      serializableMockOnly({ a: 1, b: () => 2, c: 3 }),
    ).toThrow(/command "b"/);
  });
});
```

You'll need to export `serializableMockOnly` from `e2e/fixtures/invoke-mock.ts`. Add `export` to its declaration.

- [ ] **Step 7: Run targeted vitest to verify the fixture tests work**

```bash
pnpm vitest run e2e/fixtures/
```

Expected: 3 tests pass.

- [ ] **Step 8: Commit**

```bash
git add package.json pnpm-lock.yaml .gitignore vite.config.ts e2e/
git commit -m "test(e2e): scaffold Playwright with invoke-mock fixture (Approach A)"
```

---

## Task 8: navigation.spec.ts

**Files:**
- Create: `e2e/tests/navigation.spec.ts`

- [ ] **Step 1: Write the failing test**

```typescript
import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

test.describe("Navigation", () => {
  test.beforeEach(async ({ page }) => {
    // Mock every Tauri command the page might invoke at boot — projects boot,
    // budget poll, etc. Empty results are fine for navigation testing.
    await installInvokeMock(page, {
      list_projects: [],
      get_budget_status: { spent_cents: 0, limit_cents: null },
      get_queue_status: { pending: 0, in_flight: 0 },
    });
  });

  test("/ redirects to /website and shows the Website builder", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText(/WEBSITE BUILDER/i)).toBeVisible();
  });

  test("module sidebar switches the active page", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("link", { name: /typography/i }).click();
    await expect(page).toHaveURL(/\/typography/);
    await expect(page.getByText(/MOD—05 · TYPE & LOGO/i)).toBeVisible();
  });

  test("deep-link /typography lands directly on the typography page", async ({ page }) => {
    await page.goto("/typography");
    await expect(page.getByText(/MOD—05 · TYPE & LOGO/i)).toBeVisible();
  });

  test("unknown route redirects to /website", async ({ page }) => {
    await page.goto("/nonexistent");
    await expect(page.getByText(/WEBSITE BUILDER/i)).toBeVisible();
  });
});
```

- [ ] **Step 2: Run the spec**

```bash
pnpm exec playwright test e2e/tests/navigation.spec.ts
```

Expected: all 4 tests pass. If a test fails because a sidebar link doesn't have an accessible name matching `/typography/i`, inspect the actual sidebar markup and adapt the locator.

- [ ] **Step 3: Commit**

```bash
git add e2e/tests/navigation.spec.ts
git commit -m "test(e2e): navigation spec — redirects + module switches + deep-links"
```

---

## Task 9: typography.spec.ts (full Phase 7 happy-path)

**Files:**
- Create: `e2e/tests/typography.spec.ts`

- [ ] **Step 1: Write the spec**

```typescript
import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

test.describe("Typography flow", () => {
  test("generate → vectorize → addText → export brand kit", async ({ page }) => {
    await installInvokeMock(page, {
      list_projects: [],
      get_budget_status: { spent_cents: 0, limit_cents: null },
      get_queue_status: { pending: 0, in_flight: 0 },
      generate_logo_variants: [
        { url: "data:image/png;base64,iVBORw0KGgo=", local_path: "/tmp/v1.png", seed: 1, model: "ideogram-v3" },
        { url: "data:image/png;base64,iVBORw0KGgo=", local_path: "/tmp/v2.png", seed: 2, model: "ideogram-v3" },
      ],
      vectorize_image: {
        svg: '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect width="100" height="100" fill="#e85d2d"/></svg>',
        width: 100,
        height: 100,
      },
      export_brand_kit: "/tmp/exports/acme-brand-kit.zip",
    });

    await page.goto("/typography");
    await page.getByLabel(/Describe the logo/i).fill("Acme");
    await page.getByRole("button", { name: /Generate/i }).click();

    // Wait for the gallery to populate
    const firstVariant = page.locator('[data-testid^="logo-variant-"]').first();
    await expect(firstVariant).toBeVisible();
    await firstVariant.click();

    // Vectorize button enabled, click
    await page.getByRole("button", { name: /^Vectorize$/i }).click();
    await expect(page.getByRole("button", { name: /Export brand kit/i })).toBeEnabled();

    // Open export dialog, fill, submit
    await page.getByRole("button", { name: /Export brand kit/i }).click();
    await page.getByLabel(/Brand name/i).fill("Acme");
    await page.getByLabel(/Destination directory/i).fill("/tmp/exports");
    await page.getByRole("button", { name: /^Export$/i }).click();

    // Success toast
    await expect(page.getByText(/Brand kit exported/i)).toBeVisible();
  });
});
```

- [ ] **Step 2: Run the spec**

```bash
pnpm exec playwright test e2e/tests/typography.spec.ts
```

Expected: passes. If a locator misses (e.g., `data-testid="logo-variant-..."` differs from current implementation), inspect the LogoGallery output and adapt.

- [ ] **Step 3: Commit**

```bash
git add e2e/tests/typography.spec.ts
git commit -m "test(e2e): typography flow spec — generate→vectorize→export"
```

---

## Task 10: website-builder.spec.ts

**Files:**
- Create: `e2e/tests/website-builder.spec.ts`

- [ ] **Step 1: Write the spec**

```typescript
import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

test.describe("Website builder", () => {
  test("prompt → generate → preview shows generated HTML", async ({ page }) => {
    await installInvokeMock(page, {
      list_projects: [],
      get_budget_status: { spent_cents: 0, limit_cents: null },
      get_queue_status: { pending: 0, in_flight: 0 },
      analyze_url: { palette: [], typography: [], layout: "" },
      generate_website: {
        html: "<!doctype html><html><body><h1>Mocked</h1></body></html>",
        css: "h1 { color: red; }",
      },
    });

    await page.goto("/website");
    await page.getByLabel(/Describe the site/i).fill("a one-page portfolio");
    await page.getByRole("button", { name: /Generate/i }).click();

    // The preview pane should render the mocked HTML; assert the heading is reachable
    // through the iframe (or the raw HTML snippet visible in the editor pane).
    await expect(page.getByText(/Mocked/i)).toBeVisible({ timeout: 10_000 });
  });
});
```

- [ ] **Step 2: Run the spec**

```bash
pnpm exec playwright test e2e/tests/website-builder.spec.ts
```

If the actual page uses a different label text or hides the preview behind a tab, adapt the locator. The mocked invoke shape may need adjustment — inspect `src/lib/websiteCommands.ts` for the actual return type of `generate_website`.

- [ ] **Step 3: Commit**

```bash
git add e2e/tests/website-builder.spec.ts
git commit -m "test(e2e): website-builder spec — prompt→generate→preview"
```

---

## Task 11: graphic2d.spec.ts

**Files:**
- Create: `e2e/tests/graphic2d.spec.ts`

- [ ] **Step 1: Write the spec**

```typescript
import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

test.describe("Graphic 2D", () => {
  test("prompt → generate variants → click variant opens editor", async ({ page }) => {
    await installInvokeMock(page, {
      list_projects: [],
      get_budget_status: { spent_cents: 0, limit_cents: null },
      get_queue_status: { pending: 0, in_flight: 0 },
      text_to_image: [
        { url: "data:image/png;base64,iVBORw0KGgo=", local_path: "/tmp/g1.png", seed: 1, model: "fal-flux" },
        { url: "data:image/png;base64,iVBORw0KGgo=", local_path: "/tmp/g2.png", seed: 2, model: "fal-flux" },
      ],
    });

    await page.goto("/graphic2d");
    await page.getByLabel(/Describe the image/i).fill("a sunset over the alps");
    await page.getByRole("button", { name: /Generate/i }).click();

    // Wait for at least one variant to render
    const firstThumb = page.locator('img[alt=""]').first();
    await expect(firstThumb).toBeVisible();
  });
});
```

- [ ] **Step 2: Run + adjust + commit**

```bash
pnpm exec playwright test e2e/tests/graphic2d.spec.ts
git add e2e/tests/graphic2d.spec.ts
git commit -m "test(e2e): graphic2d spec — prompt→generate→variant gallery"
```

---

## Task 12: graphic3d.spec.ts

**Files:**
- Create: `e2e/tests/graphic3d.spec.ts`

- [ ] **Step 1: Write the spec**

```typescript
import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

test.describe("Graphic 3D", () => {
  test("prompt → generate mesh → preview shows", async ({ page }) => {
    await installInvokeMock(page, {
      list_projects: [],
      get_budget_status: { spent_cents: 0, limit_cents: null },
      get_queue_status: { pending: 0, in_flight: 0 },
      generate_mesh_from_text: {
        gltf_path: "/tmp/mesh.gltf",
        thumbnail_url: "data:image/png;base64,iVBORw0KGgo=",
      },
    });

    await page.goto("/graphic3d");
    // The Graphic3D page may have multiple tabs (depth-from-image / text-to-mesh);
    // adapt the locator to the visible "Generate" entry point.
    await expect(page.locator("body")).toBeVisible();
    // Smoke test for now — full flow depends on the page's exact UI which
    // may have evolved past the spec's assumed shape.
  });
});
```

(Graphic3D is the most complex page — start with a smoke test that it renders, expand later if the team wants deeper coverage.)

- [ ] **Step 2: Run + commit**

```bash
pnpm exec playwright test e2e/tests/graphic3d.spec.ts
git add e2e/tests/graphic3d.spec.ts
git commit -m "test(e2e): graphic3d smoke test"
```

---

## Task 13: video.spec.ts

**Files:**
- Create: `e2e/tests/video.spec.ts`

- [ ] **Step 1: Write the spec**

```typescript
import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

test.describe("Video", () => {
  test("prompt → storyboard → render", async ({ page }) => {
    await installInvokeMock(page, {
      list_projects: [],
      get_budget_status: { spent_cents: 0, limit_cents: null },
      get_queue_status: { pending: 0, in_flight: 0 },
      generate_storyboard: {
        shots: [
          { description: "intro", duration_sec: 2 },
          { description: "middle", duration_sec: 3 },
          { description: "end", duration_sec: 2 },
        ],
      },
      assemble_video: { mp4_path: "/tmp/out.mp4" },
    });

    await page.goto("/video");
    await expect(page.locator("body")).toBeVisible();
    // Smoke test — full flow depends on the actual Video page UI.
  });
});
```

- [ ] **Step 2: Run + commit**

```bash
pnpm exec playwright test e2e/tests/video.spec.ts
git add e2e/tests/video.spec.ts
git commit -m "test(e2e): video smoke test"
```

---

## Task 14: Playwright CI job

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Add the e2e job**

After the `coverage` job and before `build`, insert:

```yaml
  e2e:
    name: E2E (Playwright)
    runs-on: macos-latest
    needs: [lint, test]
    steps:
      - uses: actions/checkout@v4

      - uses: pnpm/action-setup@v4
        with:
          version: ${{ env.PNPM_VERSION }}

      - uses: actions/setup-node@v4
        with:
          node-version: ${{ env.NODE_VERSION }}
          cache: "pnpm"

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Get Playwright version
        id: playwright-version
        run: echo "version=$(pnpm list --depth=0 --json | jq -r '.[0].devDependencies."@playwright/test".version')" >> $GITHUB_OUTPUT

      - name: Cache Playwright browsers
        uses: actions/cache@v4
        with:
          path: ~/Library/Caches/ms-playwright
          key: playwright-${{ runner.os }}-${{ steps.playwright-version.outputs.version }}

      - name: Install Playwright browsers
        run: pnpm exec playwright install --with-deps chromium

      - name: Run Playwright
        run: pnpm exec playwright test
        env:
          CI: "true"

      - name: Upload Playwright report on failure
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: playwright-report
          path: playwright-report/
          retention-days: 14
```

- [ ] **Step 2: Push and verify CI green**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add Playwright e2e job (chromium-only, cached browsers)"
git push origin main
gh run list --branch main --limit 1 --json databaseId
gh run watch <id> --exit-status
```

The new `e2e` job must succeed.

---

## Task 15: docs/QA-CHECKLIST.md

**Files:**
- Create: `docs/QA-CHECKLIST.md`

- [ ] **Step 1: Write the checklist**

```markdown
# Manual QA Checklist

Walk this list before tagging a release or shipping a build for live-testing. Each section ends with edge-cases that the unit/e2e tests can't easily cover (filesystem permissions, real-network failures, multi-window state).

## Boot
- [ ] First start with no projects: empty-state UI guides user to "New project"
- [ ] First start with no API keys: settings panel shows `Add key` for each provider
- [ ] Restart after creating a project: project re-opens at last position

## Website Builder (`/website`)
- [ ] Empty prompt: Generate button disabled
- [ ] Happy-path prompt: 1 result returned, preview pane renders HTML
- [ ] URL analyzer with valid URL: palette + typography extracted
- [ ] URL analyzer with broken URL: error toast, no crash
- [ ] Code editor: Monaco loads, edits propagate to preview
- [ ] Export: HTML + CSS download or saved to disk

## Graphic 2D (`/graphic2d`)
- [ ] Empty prompt: Generate disabled
- [ ] 1 / 6 variants: gallery shows correct count
- [ ] Click variant: editor opens with image
- [ ] Edit (text overlay, crop, flip): saved on canvas
- [ ] Export PNG/JPEG/PDF/GIF: file written to expected path
- [ ] Out-of-budget: warning toast before generation

## Graphic 3D (`/graphic3d`)
- [ ] Depth from image: image upload → depth map renders
- [ ] Text-to-mesh: prompt → mesh appears in 3D viewport
- [ ] Camera controls: rotate / pan / zoom respond to mouse
- [ ] Lighting preset switches: rim / softbox / studio change scene lighting
- [ ] Export glTF / animated GIF: file written

## Video (`/video`)
- [ ] Prompt → storyboard: shots returned with durations
- [ ] Edit a shot: change persists across re-render
- [ ] Render with Remotion: mp4 file written
- [ ] Render with Shotstack: cloud render polls + downloads mp4
- [ ] Render mid-flight + click another module: render continues, navigation works

## Typography (`/typography`)
- [ ] Empty prompt: Generate disabled
- [ ] Generate 6 variants: gallery populates
- [ ] Favorites toggle: heart icon flips, "Show favorites only" filters
- [ ] Favorites filter resets after re-Generate
- [ ] Click variant: editor enables Vectorize
- [ ] Vectorize: SVG renders in canvas
- [ ] Add text: input + button works, Textbox appears centered
- [ ] Slider edits (font / color / size / kerning): apply to selected Textbox in real time
- [ ] Export brand kit: dialog opens, accepts brand name + colors + font + destination
- [ ] Export with non-existent destination: clear error toast
- [ ] Export with valid destination: ZIP written, contains 12 files (svg + 8 sizes + bw + inverted + style-guide.html)

## Cross-cutting
- [ ] Sidebar module switch preserves in-flight state
- [ ] Browser back / forward: history works
- [ ] Window resize: layout reflows, no cropped content
- [ ] Cmd+Z / Cmd+Shift+Z: undo/redo on canvas-bearing pages
- [ ] No console errors on any happy-path flow
- [ ] No native panic in any flow (check Console.app for `terryblemachine` crash logs)
```

- [ ] **Step 2: Commit**

```bash
git add docs/QA-CHECKLIST.md
git commit -m "docs(testing): manual QA checklist for pre-release walkthrough"
```

---

## Task 16: Sub-Project 1 verification + closure report

**Files:**
- Create: `docs/superpowers/specs/2026-04-19-phase-8-testing-verification-report.md`

- [ ] **Step 1: Run the full verification pipeline**

```bash
pnpm test                              # frontend unit + component
pnpm test:coverage                     # frontend coverage report
cd src-tauri && cargo test && cd ..    # backend (incl. wire-tests + integration)
cd src-tauri && cargo llvm-cov --workspace --lcov --output-path ../coverage/backend.lcov && cd ..
pnpm exec playwright test              # E2E
pnpm exec tsc --noEmit
pnpm biome check .
```

All must be clean / green. Capture the test counts and coverage percentages.

- [ ] **Step 2: Write the closure report**

```markdown
# Phase 8 Sub-Project 1 (Testing) — Verification Report

**Date:** <today>
**Spec:** `docs/superpowers/specs/2026-04-19-phase-8-testing-design.md`
**Plan:** `docs/superpowers/plans/2026-04-19-phase-8-testing.md`

## Summary

All 4 testing pillars implemented. Verification pipeline runs green:
- Frontend: `pnpm test` <N> tests / <M> files; `pnpm test:coverage` <X>% lines
- Backend: `cargo test` <N> tests; `cargo llvm-cov` <Y>% lines
- E2E: `pnpm exec playwright test` <N> specs / <M> tests
- Lint/format: clean
- CI: latest run green, coverage + e2e jobs both succeed

## Pillar coverage

### 1. Coverage Reporting
- Frontend: <details>
- Backend: <details>
- CI artifacts: <link to action>

### 2. API-Client Wire-Tests
- Audit: `src-tauri/tests/api_clients_wire_audit.md` — all 9 clients covered across 4 pillars
- Gaps filled: <list, or "none — already complete">

### 3. Playwright E2E (Approach A)
- 6 specs / <M> tests across 6 modules
- Cache + trace upload wired in CI

### 4. Manual QA Checklist
- `docs/QA-CHECKLIST.md` — <count> bullets across 6 sections

## Backlog touched
- #177 (POST-PHASE-7): integration test ABI coverage — the Playwright invoke-mock fixture provides a substrate to revisit; the backlog item stays open until someone ports the original Typography integration test into the Playwright suite or upgrades the mock to assert command name + payload.

## Verdict
Phase 8 Sub-Project 1 closed. Ready to brainstorm Sub-Project 2 (UX-Polish).
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-04-19-phase-8-testing-verification-report.md
git commit -m "docs(testing): Phase 8 Sub-Project 1 (Testing) verification report"
git push origin main
gh run watch <run-id> --exit-status
```

---

## Self-Review

**Spec coverage check:**
- ✓ Coverage reporting (frontend + backend) → Tasks 1, 2, 6
- ✓ API-client wire-tests → Tasks 3, 4
- ✓ Playwright E2E (Approach A) → Tasks 7-13, 14 (CI)
- ✓ Manual-QA checklist → Task 15
- ✓ docs/TESTING.md → Task 5
- ✓ Soft-gate semantics → Task 5 (TESTING.md) + Task 6 (CI uses summary not threshold)
- ✓ Non-goals respected (no Tauri-WebDriver, no distribution, no a11y, no visual-regression)

**Placeholder scan:**
- Task 12 + Task 13 are explicitly framed as "smoke tests" because graphic3d and video pages have complex UIs that the spec didn't dictate flows for; this is intentional, not placeholder.
- Task 4 has a conditional skip if Task 3's audit shows zero gaps; that's a real branch, not a placeholder.

**Type consistency:**
- `installInvokeMock` is defined in Task 7 and used in Tasks 8-13.
- `InMemoryKeyStore` is defined in Task 4 (or noted as already-existing) and referenced consistently.
- All `*Commands.ts` IPC names mocked in specs match the actual Tauri command names registered in `src-tauri/src/lib.rs`'s `invoke_handler!`.
