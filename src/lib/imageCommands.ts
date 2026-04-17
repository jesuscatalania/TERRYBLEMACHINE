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
