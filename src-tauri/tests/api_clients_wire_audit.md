# API-Client Wiremock Coverage Audit

**Date:** 2026-04-19
**Scope:** 9 API clients in `src-tauri/src/api_clients/`. Inline `#[cfg(test)]` blocks audited for the four wire-test pillars.

## Methodology

Each of the 9 clients was inspected for inline unit tests using `wiremock::MockServer`.
Tests were categorised into four pillars:

- **Success** — asserts canonical happy-path JSON parses to the expected typed
  `AiResponse` (or equivalent domain type).
- **Auth** — asserts the outgoing request carries the provider-specific
  authorization header AND asserts that a missing/empty key yields
  `ProviderError::Auth`. Both must be true to count as ✓.
- **Error-status** — asserts HTTP 4xx / 5xx maps to the correct variant
  (`Auth` / `Transient` / `Permanent`) via at least one 5xx test AND one 4xx test.
- **Timeout** — asserts that response delay / exhausted poll attempts / no
  terminal status maps to `ProviderError::Timeout`. For synchronous
  request/response clients (no polling) this pillar covers the reqwest
  60s HTTP timeout path (`map_reqwest_error` → `Timeout`).

## Matrix

| Client | Success | Auth | Error | Timeout | Gaps |
|---|---|---|---|---|---|
| claude | ✓ | ✓ | ✓ | ✗ | timeout |
| kling | ✓ | ✓ | ✓ | ✗ | timeout |
| runway | ✓ | ✓ | ✓ | ✓ | — |
| higgsfield | ✓ | ✓ | ✓ | ✓ | — |
| shotstack | ✓ | ✓ | ✓ | ✓ | — |
| ideogram | ✓ | ✓ | ✓ | ✗ | timeout |
| meshy | ✓ | ✓ | ✓ | ✓ | — |
| fal | ✓ | ✓ | ✓ | ✗ | timeout |
| replicate | ✓ | ✓ | ✓ | ✗ | timeout (polling) |

## Evidence

### claude (`src-tauri/src/api_clients/claude.rs`)

- Success: `happy_path_returns_text_output` — asserts parsed text output.
- Auth: `happy_path_returns_text_output` (mock requires `x-api-key: sk-test`
  and `anthropic-version` headers) + `missing_key_yields_auth_error`.
- Error: `server_500_is_transient` + `status_401_is_auth` +
  `unsupported_model_is_permanent` + `vision_payload_rejects_missing_media_type`.
- Timeout: **no test**. Synchronous POST — reqwest 60s path uncovered.

### kling (`src-tauri/src/api_clients/kling.rs`)

- Success: `happy_path_returns_task_job` + `image_to_video_hits_image2video_endpoint`.
- Auth: `happy_path_returns_task_job` (mock requires `Authorization: Bearer kling-test-key`)
  + `missing_key_yields_auth_error`.
- Error: `server_500_is_transient` + `status_401_is_auth`
  + `unsupported_model_is_permanent` + `image_to_video_requires_image_url`
  + `unsupported_task_is_permanent`.
- Timeout: **no test**. Single-shot submit (no polling) — reqwest timeout path uncovered.

### runway (`src-tauri/src/api_clients/runway.rs`)

- Success: `happy_path_returns_video_url` + `runway_text_to_video_polls_until_succeeded`
  + `motion_brush_strokes_forwarded`.
- Auth: `happy_path_returns_video_url` (mock requires `Authorization: Bearer runway-test-key`)
  + `missing_key_yields_auth_error`.
- Error: `server_500_is_transient` + `status_401_is_auth`
  + `unsupported_model_is_permanent` + `runway_propagates_failed_status`
  + `motion_brush_scalar_is_rejected`.
- Timeout: `runway_times_out_after_max_attempts` — asserts polling exhaustion
  returns `ProviderError::Timeout`.

### higgsfield (`src-tauri/src/api_clients/higgsfield.rs`)

- Success: `happy_path_returns_video_url` + `higgsfield_text_to_video_polls_until_succeeded`.
- Auth: `happy_path_returns_video_url` (mock requires `x-api-key: higgs-test-key`)
  + `missing_key_yields_auth_error`.
- Error: `server_500_is_transient` + `status_401_is_auth`
  + `unsupported_model_is_permanent` + `higgsfield_propagates_failed_status`.
- Timeout: `higgsfield_times_out_after_max_attempts`.

### shotstack (`src-tauri/src/api_clients/shotstack.rs`)

- Success: `happy_path_returns_job_id` + `shotstack_assembly_posts_timeline_and_returns_id`
  + `shotstack_assembly_polls_until_done` + `prod_env_posts_to_v1_render_path`
  + `prod_env_polls_v1_render_path`.
- Auth: `happy_path_returns_job_id` (mock requires `x-api-key: sk-test`)
  + `missing_key_yields_auth_error`.
- Error: `server_500_is_transient` + `status_401_is_auth`
  + `unsupported_model_is_permanent` + `shotstack_assembly_propagates_failed_status`
  + `shotstack_assembly_missing_id_is_permanent` + `build_body_does_not_leak_user_prompt`.
- Timeout: `shotstack_assembly_times_out_after_max_attempts`.

### ideogram (`src-tauri/src/api_clients/ideogram.rs`)

- Success: `happy_path_returns_url` + `v3_model_version_sent` + `v3_model_version_in_image_request`.
- Auth: `happy_path_returns_url` (mock requires `Api-Key: sk-test`)
  + `missing_key_yields_auth_error`.
- Error: `server_500_is_transient` + `status_401_is_auth`
  + `unsupported_model_is_permanent`.
- Timeout: **no test**. Synchronous POST — reqwest timeout path uncovered.

### meshy (`src-tauri/src/api_clients/meshy.rs`)

- Success: `happy_path_text_to_3d_returns_job_id`
  + `text_to_3d_polls_until_succeeded_then_returns_glb_url`
  + `happy_path_image_to_3d_returns_job_id`
  + `image_to_3d_polls_until_succeeded_then_returns_glb_url`.
- Auth: `happy_path_text_to_3d_returns_job_id` + `happy_path_image_to_3d_returns_job_id`
  (both mocks require `authorization: Bearer sk-test`) + `missing_key_yields_auth_error`.
- Error: `server_500_is_transient` + `status_401_is_auth`
  + `unsupported_model_is_permanent` + `text_to_3d_propagates_failed_status`.
- Timeout: `text_to_3d_times_out_after_max_attempts`.

### fal (`src-tauri/src/api_clients/fal.rs`)

- Success: `flux_pro_happy_path` + `sdxl_happy_path`
  + `real_esrgan_happy_path` + `flux_fill_happy_path`.
- Auth: all four happy-path tests (mocks require `authorization: Key fal-test`)
  + `missing_key_yields_auth_error`.
- Error: `server_500_is_transient` + `status_401_is_auth`
  + `unsupported_model_is_permanent`.
- Timeout: **no test**. Synchronous POST-then-await — reqwest timeout path uncovered.

### replicate (`src-tauri/src/api_clients/replicate.rs`)

- Success: `happy_path_returns_prediction`
  + `replicate_polls_starting_prediction_until_succeeded`
  + `depth_anything_v2_posts_to_slug_endpoint` + `triposr_posts_to_slug_endpoint`.
- Auth: `happy_path_returns_prediction` (mock requires
  `authorization: Token r8-test` + `Prefer: wait`)
  + `missing_key_yields_auth_error`.
- Error: `server_500_is_transient` + `status_401_is_auth`
  + `unsupported_model_is_permanent` + `triposr_requires_image_url`
  + `depth_anything_v2_requires_image_url`.
- Timeout: **no test**. `poll_prediction` has a `ProviderError::Timeout`
  branch on exhausted `poll_max_attempts` — uncovered by wiremock tests.

## Summary

**Total gaps to fill:** 5 — timeout pillar missing for `claude`, `kling`,
`ideogram`, `fal`, `replicate`.

The 4 long-polling clients (runway, higgsfield, shotstack, meshy) all have
explicit polling-exhaustion timeout tests. Of the remaining 5:

- `replicate` also polls (`poll_prediction`) but lacks a polling-exhaustion
  test — this is a real test-coverage gap.
- `claude`, `kling`, `ideogram`, `fal` are synchronous request/response.
  Their `ProviderError::Timeout` path arises only from the reqwest 60s HTTP
  timeout (`map_reqwest_error → Timeout`). A response-delay wiremock test
  (configured below `map_reqwest_error`'s threshold) would cover this.

T4 follow-up will fill these 5 gaps using the established `for_test` /
`MockServer` pattern (plus, for polling clients, the `for_test_with_polling`
variant that dials the poll budget down for fast tests).
