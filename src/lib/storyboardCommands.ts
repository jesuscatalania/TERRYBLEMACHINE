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
}

export interface Shot {
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

export const generateStoryboard = (input: StoryboardInput) =>
  invoke<Storyboard>("generate_storyboard", { input });
