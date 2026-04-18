import { invoke } from "@tauri-apps/api/core";

/**
 * Vectorize (raster→SVG) IPC types for the VTracer-backed backend in
 * `src-tauri/src/vectorizer/`.
 *
 * The backend `ColorMode` is a serde `kebab-case` enum with exactly two
 * variants (`"color"` | `"bw"`) — the union below must match verbatim or
 * Rust's Deserialize will reject the payload. See
 * `src-tauri/src/vectorizer/types.rs` for the source of truth.
 */
export interface VectorizeInput {
  image_path: string;
  color_mode?: "color" | "bw";
  filter_speckle?: number;
  corner_threshold?: number;
}

export interface VectorizeResult {
  svg: string;
  width: number;
  height: number;
}

export const vectorizeImage = (input: VectorizeInput) =>
  invoke<VectorizeResult>("vectorize_image", { input });
