import { invoke } from "@tauri-apps/api/core";

export interface RemotionInput {
  composition: string;
  props: Record<string, unknown>;
}
export interface RemotionResult {
  output_path: string;
  composition: string;
}

export const renderRemotion = (input: RemotionInput) =>
  invoke<RemotionResult>("render_remotion", { input });
