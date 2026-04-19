import { invoke } from "@tauri-apps/api/core";

export type StoryboardTemplate =
  | "commercial"
  | "explainer"
  | "social-media"
  | "music-video"
  | "custom";

export interface StoryboardInput {
  prompt: string;
  template?: StoryboardTemplate;
  module?: string;
  /**
   * PascalCase Model enum variant (e.g. `"FalKlingV2Master"`), or
   * undefined to let the router's strategy pick. Resolved from either
   * the Tool dropdown or a `/tool` prompt override. Sent as snake_case
   * since the Rust `StoryboardInput` struct keeps field names as-is.
   */
  model_override?: string;
}

export interface Shot {
  /**
   * Stable client-side id used as React key. The Rust backend does NOT send
   * this field; we generate one on ingest (see {@link ensureShotIds}) so that
   * reordering shots — which renumbers `index` — does not remount every card
   * mid-edit and blur an active input.
   */
  id: string;
  index: number;
  description: string;
  style: string;
  duration_s: number;
  camera: string;
  transition: string;
}

export interface Storyboard {
  summary: string;
  template: string;
  shots: Shot[];
}

/**
 * Assign a stable `id` to any shot that lacks one. Used when ingesting a
 * Storyboard from the Rust backend, whose schema currently has no id field.
 */
export function ensureShotIds(sb: Storyboard): Storyboard {
  return {
    ...sb,
    shots: sb.shots.map((s) => (s.id ? s : { ...s, id: crypto.randomUUID() })),
  };
}

export const generateStoryboard = (input: StoryboardInput) =>
  invoke<Storyboard>("generate_storyboard", { input }).then(ensureShotIds);
