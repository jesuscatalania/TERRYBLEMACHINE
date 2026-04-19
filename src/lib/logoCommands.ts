import { invoke } from "@tauri-apps/api/core";

export type LogoStyle = "minimalist" | "wordmark" | "emblem" | "mascot";

export interface LogoInput {
  prompt: string;
  style?: LogoStyle;
  count?: number;
  palette?: string;
  module?: string;
  /**
   * PascalCase Model enum variant (e.g. `"IdeogramV3"`), or undefined to
   * let the router's strategy pick. Resolved from either the Tool
   * dropdown or a `/tool` prompt override. Sent as snake_case since the
   * Rust `LogoInput` struct keeps field names as-is.
   */
  model_override?: string;
}

export interface LogoVariant {
  url: string;
  local_path: string | null;
  seed: number | null;
  model: string;
}

export const generateLogoVariants = (input: LogoInput) =>
  invoke<LogoVariant[]>("generate_logo_variants", { input });
