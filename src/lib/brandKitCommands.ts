import { invoke } from "@tauri-apps/api/core";

/**
 * Mirrors `BrandKitInput` from `src-tauri/src/brand_kit/types.rs`.
 *
 * Field names (`logo_svg`, `source_png_path`, etc.) match the Rust struct
 * verbatim — the backend uses the default serde naming strategy, so the
 * frontend must ship snake_case keys. Don't "fix" these to camelCase
 * without matching the backend.
 */
export interface BrandKitInput {
  logo_svg: string;
  source_png_path: string;
  brand_name: string;
  primary_color: string;
  accent_color: string;
  font: string;
}

/**
 * Mirrors the kebab-case tagged enum `BrandKitIpcError` from
 * `src-tauri/src/brand_kit/commands.rs`. Use `isBrandKitIpcError` before
 * reading `.detail` — `invoke` rejections are typed as `unknown`.
 */
export interface BrandKitIpcError {
  kind: "invalid-input" | "image" | "io";
  detail: string;
}

export function isBrandKitIpcError(err: unknown): err is BrandKitIpcError {
  return (
    typeof err === "object" &&
    err !== null &&
    "kind" in err &&
    "detail" in err &&
    typeof (err as { kind: unknown }).kind === "string" &&
    typeof (err as { detail: unknown }).detail === "string"
  );
}

/**
 * Builds every brand-kit asset and writes a single ZIP to `destination`.
 * Returns the absolute path of the written archive.
 */
export const exportBrandKit = (input: BrandKitInput, destination: string) =>
  invoke<string>("export_brand_kit", { input, destination });
