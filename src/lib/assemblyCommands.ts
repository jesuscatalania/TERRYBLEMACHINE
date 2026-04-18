import { invoke } from "@tauri-apps/api/core";

/**
 * Single clip on the Shotstack timeline. Keys are snake_case to match the
 * Rust `AssemblyClip` serde definition in `shotstack_assembly/types.rs`.
 * `transition_in` / `transition_out` are passed through to Shotstack verbatim;
 * valid names live in the Shotstack transition catalogue.
 */
export interface AssemblyClip {
  src: string;
  start_s: number;
  length_s: number;
  transition_in?: string;
  transition_out?: string;
}

/**
 * Input for the `assemble_video` Tauri command. Mirrors the Rust
 * `AssemblyInput` struct — `format` and `resolution` are optional on the wire
 * because the backend applies serde defaults ("mp4" / "hd") when absent.
 */
export interface AssemblyInput {
  clips: AssemblyClip[];
  soundtrack?: string;
  format?: "mp4" | "gif";
  resolution?: "sd" | "hd" | "1080";
}

/**
 * Result returned by the `assemble_video` Tauri command. `local_path` is
 * populated when the backend successfully cached the rendered MP4; frontends
 * should prefer it (via Tauri's `convertFileSrc`) and fall back to
 * `video_url` when `null`.
 */
export interface AssemblyResult {
  render_id: string;
  video_url: string;
  local_path: string | null;
}

/**
 * Thin wrapper around `invoke("assemble_video")`. The backend posts the
 * timeline to Shotstack, polls to `done`, and best-effort caches the MP4
 * locally before resolving.
 */
export const assembleVideo = (input: AssemblyInput) =>
  invoke<AssemblyResult>("assemble_video", { input });
