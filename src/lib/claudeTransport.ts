import { invoke } from "@tauri-apps/api/core";

/**
 * Transport channel used to talk to Claude.
 * - `auto`: prefer the locally-installed `claude` CLI (subscription billing),
 *   fall back to the HTTP API when the binary is missing.
 * - `api`: always use the HTTP API (requires an `ANTHROPIC_API_KEY` in the
 *   keychain under `claude`).
 * - `cli`: always use the CLI — errors out at registry build time if the
 *   binary isn't installed.
 */
export type ClaudeTransport = "auto" | "api" | "cli";

/**
 * Ask the backend whether a `claude` CLI binary is installed. Returns the
 * absolute path when found, or `null` otherwise.
 */
export function detectClaudeCli(): Promise<string | null> {
  return invoke<string | null>("detect_claude_cli");
}

/** Read the persisted transport selection. */
export function getClaudeTransport(): Promise<ClaudeTransport> {
  return invoke<ClaudeTransport>("get_claude_transport");
}

/** Persist the user's transport selection. */
export function setClaudeTransport(transport: ClaudeTransport): Promise<void> {
  return invoke<void>("set_claude_transport", { transport });
}
