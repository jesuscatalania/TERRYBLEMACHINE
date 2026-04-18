import { invoke } from "@tauri-apps/api/core";

/**
 * Input for the `generate_depth` Tauri command. Keys are snake_case to match
 * the Rust `DepthInput` serde definition in `depth_pipeline/types.rs`.
 */
export interface DepthInput {
  image_url: string;
  module?: string;
}

/**
 * Result returned by the `generate_depth` Tauri command. Fields mirror the
 * Rust `DepthResult` struct and arrive with default (snake_case) serde.
 */
export interface DepthResult {
  depth_url: string;
  model: string;
  cached: boolean;
}

/**
 * Thin wrapper around `invoke("generate_depth")`. The backend pipeline runs
 * through the AI router (Replicate's depth-anything v2 endpoint) and returns
 * a URL pointing at the single-channel depth PNG.
 */
export function generateDepth(input: DepthInput): Promise<DepthResult> {
  return invoke<DepthResult>("generate_depth", { input });
}
