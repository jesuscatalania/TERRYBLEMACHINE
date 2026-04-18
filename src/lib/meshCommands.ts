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
 *
 * `quick_preview` opts into the TripoSR tier (faster + cheaper, lower
 * fidelity). When omitted or `false`, the backend stays on Meshy.
 */
export interface MeshImageInput {
  image_url: string;
  prompt?: string;
  module?: string;
  quick_preview?: boolean;
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

/**
 * Copy a cached GLB at `localPath` to `targetPath` (absolute paths on both
 * sides). Tauri auto-converts the camelCase keys to the Rust command's
 * snake_case parameters. Parent dirs are created on the Rust side; missing
 * source surfaces as an `InvalidInput` error the caller can toast.
 */
export function exportMesh(localPath: string, targetPath: string): Promise<string> {
  return invoke<string>("export_mesh", { localPath, targetPath });
}
