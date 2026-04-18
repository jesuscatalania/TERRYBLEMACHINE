import { invoke } from "@tauri-apps/api/core";

/**
 * Input for the `generate_mesh_from_text` Tauri command. Keys are snake_case
 * to match the Rust `MeshTextInput` serde definition in
 * `mesh_pipeline/types.rs`.
 */
export interface MeshTextInput {
  prompt: string;
  module?: string;
}

/**
 * Input for the `generate_mesh_from_image` Tauri command. Keys are snake_case
 * to match the Rust `MeshImageInput` serde definition.
 */
export interface MeshImageInput {
  image_url: string;
  prompt?: string;
  module?: string;
}

/**
 * Result returned by either mesh command. `local_path` is populated when the
 * backend succeeded in downloading the GLB into the platform cache dir;
 * frontends should prefer it (via Tauri's `convertFileSrc`) and fall back
 * to `glb_url` when `null`.
 */
export interface MeshResult {
  glb_url: string;
  local_path: string | null;
  model: string;
}

/**
 * Text-to-3D via Meshy. Backend routes through the AI router, polls the
 * provider to completion, and caches the resulting GLB locally.
 */
export function generateMeshFromText(input: MeshTextInput): Promise<MeshResult> {
  return invoke<MeshResult>("generate_mesh_from_text", { input });
}

/**
 * Image-to-3D via Meshy. `image_url` must be hosted — data-URLs are rejected
 * at the pipeline boundary.
 */
export function generateMeshFromImage(input: MeshImageInput): Promise<MeshResult> {
  return invoke<MeshResult>("generate_mesh_from_image", { input });
}
