import { invoke } from "@tauri-apps/api/core";

// ─── Types mirror the Rust side ────────────────────────────────────────

export interface AssetDownload {
  url: string;
  saved_as: string;
}

export interface DetectedFeatures {
  has_canvas?: boolean;
  has_video?: boolean;
  has_form?: boolean;
  has_iframe?: boolean;
  has_webgl?: boolean;
  has_three_js?: boolean;
}

export interface TypographyStyle {
  size: string;
  weight: string;
  family: string;
}

export interface ColorRoles {
  bg?: string | null;
  fg?: string | null;
  accent?: string | null;
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

  // Deep-analysis fields (all optional — older cached analyses may omit them).
  hero_text?: string | null;
  nav_items?: string[];
  section_headings?: string[];
  paragraph_sample?: string[];
  cta_labels?: string[];
  detected_features?: DetectedFeatures;
  typography?: TypographyStyle[];
  image_urls?: string[];
  color_roles?: ColorRoles;
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
  /**
   * PascalCase Model enum variant (e.g. `"ClaudeSonnet"`), or undefined to
   * let the router's strategy pick. Resolved from either the Tool
   * dropdown or a `/tool` prompt override. Sent as snake_case since the
   * Rust `GenerationInput` struct keeps field names as-is.
   */
  model_override?: string;
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

// ─── Open in system default browser ────────────────────────────────────

/**
 * Writes the given project out to a fresh temp directory and opens its
 * `index.html` in the system default browser. Returns the `file://` URL
 * that was opened (handy for toasts/logs).
 */
export function openInBrowser(project: GeneratedProject): Promise<string> {
  return invoke<string>("open_project_in_browser", { project });
}

// ─── Refine flow ───────────────────────────────────────────────────────

export interface RefineResult {
  project: GeneratedProject;
  changed_paths: string[];
}

/**
 * Iteratively refine an existing project with a free-text instruction.
 * The backend sends the full current project + the instruction to Claude;
 * Claude returns only the files that changed (empty content = deletion)
 * and the backend merges them on top of the current file set.
 */
export function refineWebsite(
  project: GeneratedProject,
  instruction: string,
): Promise<RefineResult> {
  return invoke<RefineResult>("refine_website", {
    input: { project, instruction },
  });
}
