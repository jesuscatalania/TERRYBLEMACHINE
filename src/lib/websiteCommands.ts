import { invoke } from "@tauri-apps/api/core";

// ─── Types mirror the Rust side ────────────────────────────────────────

export interface AssetDownload {
  url: string;
  saved_as: string;
}

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
  assets?: AssetDownload[];
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
export type DeployTarget = "vercel" | "netlify";

export interface ExportRequest {
  project: GeneratedProject;
  format?: ExportFormat;
  destination: string;
  /** Optional hosting provider config to bundle (`vercel.json` / `netlify.toml`). */
  deploy?: DeployTarget;
}

// ─── Invoke wrappers ───────────────────────────────────────────────────

export function analyzeUrl(
  url: string,
  options: { screenshotPath?: string; projectPath?: string } = {},
): Promise<AnalysisResult> {
  return invoke<AnalysisResult>("analyze_url", {
    input: {
      url,
      screenshot_path: options.screenshotPath ?? null,
      project_path: options.projectPath ?? null,
    },
  });
}

export function generateWebsite(input: GenerationInput): Promise<GeneratedProject> {
  return invoke<GeneratedProject>("generate_website", { input });
}

export function exportWebsite(request: ExportRequest): Promise<string> {
  return invoke<string>("export_website", { request });
}

// ─── Inline code-assist (Cmd+K selection → replacement) ────────────────

export interface ModifyCodeSelectionInput {
  file_path: string;
  selection: string;
  instruction: string;
}

export interface ModifyCodeSelectionOutput {
  replacement: string;
}

export function modifyCodeSelection(
  req: ModifyCodeSelectionInput,
): Promise<ModifyCodeSelectionOutput> {
  return invoke<ModifyCodeSelectionOutput>("modify_code_selection", { req });
}
