import { invoke } from "@tauri-apps/api/core";

export type LogoStyle = "minimalist" | "wordmark" | "emblem" | "mascot";

export interface LogoInput {
  prompt: string;
  style?: LogoStyle;
  count?: number;
  palette?: string;
  module?: string;
}

export interface LogoVariant {
  url: string;
  local_path: string | null;
  seed: number | null;
  model: string;
}

export const generateLogoVariants = (input: LogoInput) =>
  invoke<LogoVariant[]>("generate_logo_variants", { input });
