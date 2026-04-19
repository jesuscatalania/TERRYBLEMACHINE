import { invoke } from "@tauri-apps/api/core";

export interface ImageResult {
  url: string;
  width?: number | null;
  height?: number | null;
  seed?: number | null;
  model: string;
  cached: boolean;
}

export type Complexity = "simple" | "medium" | "complex";

export interface Text2ImageInput {
  prompt: string;
  complexity?: Complexity;
  module?: string;
  /**
   * PascalCase Model enum variant (e.g. `"FalFluxPro"`), or undefined to
   * let the router's strategy pick. Resolved from either the Tool
   * dropdown or a `/tool` prompt override. Sent as snake_case since the
   * Rust input struct keeps field names as-is.
   */
  model_override?: string;
}

export interface GenerateVariantsInput extends Text2ImageInput {
  count?: number;
}

export interface Image2ImageInput extends Text2ImageInput {
  image_url: string;
}

export interface UpscaleInput {
  image_url: string;
  scale?: number;
}

export interface InpaintInput {
  prompt: string;
  source_url: string;
  mask_url: string;
  complexity?: Complexity;
  module?: string;
}

export function textToImage(input: Text2ImageInput): Promise<ImageResult> {
  return invoke<ImageResult>("text_to_image", { input });
}

export function imageToImage(input: Image2ImageInput): Promise<ImageResult> {
  return invoke<ImageResult>("image_to_image", { input });
}

export function upscaleImage(input: UpscaleInput): Promise<ImageResult> {
  return invoke<ImageResult>("upscale_image", { input });
}

export function generateVariants(input: GenerateVariantsInput): Promise<ImageResult[]> {
  return invoke<ImageResult[]>("generate_variants", { input });
}

export function inpaintImage(input: InpaintInput): Promise<ImageResult> {
  return invoke<ImageResult>("inpaint_image", { input });
}

/**
 * Returns true if the given string looks like a `data:` URL. fal.ai's
 * flux-fill endpoint requires publicly-hosted URLs; data-URLs from the
 * canvas must be rejected at the UI layer until the Phase 5 upload shim
 * ships.
 */
export function isDataUrl(url: string): boolean {
  return /^data:/i.test(url.trim());
}
