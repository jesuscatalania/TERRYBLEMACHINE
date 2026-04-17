import { invoke } from "@tauri-apps/api/core";

// ─── Types mirror the Rust side ────────────────────────────────────────

export interface AnalysisResult {
  url: string;
  status: number;
  title: string;
  description?: string | null;
  colors: string[];
  fonts: string[];
  spacing: string[];
  customProperties: Record<string, string>;
  layout: string;
  screenshotPath?: string | null;
}

export interface GeneratedFile {
  path: string;
  content: string;
}

export interface GeneratedProject {
  summary: string;
  files: GeneratedFile[];
  prompt: string;
}

export type Template = "landing-page" | "portfolio" | "blog" | "dashboard" | "ecommerce" | "custom";

export interface GenerationInput {
  prompt: string;
  template?: Template;
  reference?: AnalysisResult | null;
  image_path?: string | null;
  module?: string;
}

export type ExportFormat = "raw" | "react" | "next-js";

export interface ExportRequest {
  project: GeneratedProject;
  format?: ExportFormat;
  destination: string;
}

// ─── Invoke wrappers ───────────────────────────────────────────────────

export function analyzeUrl(url: string, screenshotPath?: string): Promise<AnalysisResult> {
  return invoke<AnalysisResult>("analyze_url", {
    input: { url, screenshot_path: screenshotPath ?? null },
  });
}

export function generateWebsite(input: GenerationInput): Promise<GeneratedProject> {
  return invoke<GeneratedProject>("generate_website", { input });
}

export function exportWebsite(request: ExportRequest): Promise<string> {
  return invoke<string>("export_website", { request });
}
