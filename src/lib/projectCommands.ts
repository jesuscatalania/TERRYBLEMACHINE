import { invoke } from "@tauri-apps/api/core";
import type { Project } from "@/stores/projectStore";

/** Mirrors `NewProject` on the Rust side. */
export interface NewProjectInput {
  name: string;
  module: Project["module"];
  description?: string;
}

export interface ProjectIpcError {
  kind: "NotFound" | "InvalidName" | "Io" | "Serde";
  detail: string;
}

export function isProjectIpcError(value: unknown): value is ProjectIpcError {
  return (
    typeof value === "object" &&
    value !== null &&
    "kind" in value &&
    typeof (value as { kind: unknown }).kind === "string"
  );
}

/** Creates a project folder + metadata on disk. */
export function createProject(input: NewProjectInput): Promise<Project> {
  return invoke<Project>("create_project", { input });
}

/** Loads a project's metadata by id (slug). */
export function openProjectFile(id: string): Promise<Project> {
  return invoke<Project>("open_project", { id });
}

/** Lists every project in the root, newest first. */
export function listProjects(): Promise<Project[]> {
  return invoke<Project[]>("list_projects");
}

/** Removes a project folder (idempotent). */
export function deleteProject(id: string): Promise<void> {
  return invoke<void>("delete_project", { id });
}

/** Absolute path of the projects root — useful for diagnostics / Settings. */
export function projectsRoot(): Promise<string> {
  return invoke<string>("projects_root");
}

/**
 * Reads the serialised undo/redo history for a project.
 * Returns an empty-stacks payload when no `history.json` exists yet.
 */
export function readProjectHistory(path: string): Promise<string> {
  return invoke<string>("read_project_history", { path });
}

/** Persists the serialised undo/redo history to `<project>/history.json`. */
export function writeProjectHistory(path: string, json: string): Promise<void> {
  return invoke<void>("write_project_history", { path, json });
}
