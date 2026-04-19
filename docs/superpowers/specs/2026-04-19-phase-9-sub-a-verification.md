# Phase 9 Sub-Project A — Claude CLI Bridge — Closure

**Date:** 2026-04-19
**Spec:** `docs/superpowers/specs/2026-04-19-phase-9-claude-bridge-and-tool-ux-design.md`
**Plan:** `docs/superpowers/plans/2026-04-19-phase-9-claude-bridge-and-tool-ux.md`

## Summary

ClaudeCliClient implemented; transport-selectable per Claude in Settings (Auto / API / CLI). All 5 backend Claude callers route through whichever transport is configured, with no other code changes (drop-in via `AiClient` trait).

Verified: cargo test 457 passed / 4 ignored; frontend vitest 421 tests across 83 files pass; pnpm tsc --noEmit clean; pnpm biome check clean; e2e 16 tests — 15 stable + 1 pre-existing flake (`Mod+1..Mod+5 switch modules`) that retries green under CI (retries=1). CI expected 5/5 green.

## Pillar coverage

- discovery: `detect_claude_binary` — 3 unit tests (candidate paths, xdg-open fallback, PATH resolution)
- stream-parser: 5 tests covering normal stream, tool-use blocks, error subtype, malformed lines, empty stream
- ClaudeCliClient: 4 mock-spawner tests (happy path, exit-code mapping, error subtype mapping, supports filter)
- Tauri commands: detect_claude_cli, get_claude_transport, set_claude_transport — registered in invoke_handler
- Registry: transport-aware Claude client construction with auto-fallback when CLI missing
- Frontend: claudeTransport.ts wrapper + Settings UI tri-state selector + CLI auto-detect display
- AiRequest.model_override: foundation for Sub-Project B's `/tool` override; router honors it

Sub-Project B (Chat panel + tool UX) follows.
