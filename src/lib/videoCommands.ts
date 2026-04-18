import { invoke } from "@tauri-apps/api/core";

export interface VideoTextInput {
  prompt: string;
  duration_s?: number;
  module?: string;
}

export interface VideoImageInput {
  image_url: string;
  prompt?: string;
  duration_s?: number;
  module?: string;
}

export interface VideoResult {
  video_url: string;
  local_path: string | null;
  model: string;
  duration_s: number | null;
}

export const generateVideoFromText = (input: VideoTextInput) =>
  invoke<VideoResult>("generate_video_from_text", { input });

export const generateVideoFromImage = (input: VideoImageInput) =>
  invoke<VideoResult>("generate_video_from_image", { input });
