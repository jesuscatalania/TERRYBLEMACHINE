import { create } from "zustand";
import { readProjectHistory, writeProjectHistory } from "@/lib/projectCommands";
import type { ModuleId } from "@/stores/appStore";
import { useHistoryStore } from "@/stores/historyStore";

export interface Project {
  id: string;
  name: string;
  module: ModuleId;
  path: string;
  createdAt: string;
  description?: string;
}

const RECENTS_LIMIT = 10;

export interface ProjectState {
  currentProject: Project | null;
  recents: Project[];
  openProject: (project: Project) => void;
  closeProject: () => void;
  addRecent: (project: Project) => void;
  /** Replace recents with a freshly-loaded list (sorted newest-first by caller). */
  hydrateRecents: (projects: readonly Project[]) => void;
  clearRecents: () => void;
}

/**
 * Fire-and-forget history hydration. Errors are swallowed — a missing or
 * corrupt `history.json` must never block opening a project. On failure the
 * caller sees an empty history (the default state after `clear()`).
 */
function hydrateHistoryFor(project: Project): void {
  readProjectHistory(project.path)
    .then((raw) => useHistoryStore.getState().hydrate(raw))
    .catch(() => {
      /* non-fatal: start with empty history */
    });
}

/**
 * Fire-and-forget history persistence. Failures are swallowed — a failed
 * write on close must never block the user's workflow.
 */
function persistHistoryFor(project: Project): void {
  const json = useHistoryStore.getState().serialize();
  writeProjectHistory(project.path, json).catch(() => {
    /* non-fatal: history may be lost for this session */
  });
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  currentProject: null,
  recents: [],
  openProject: (project) => {
    // Reset in-memory history before hydrating — prevents the previous
    // project's stacks from leaking in if hydration races or fails.
    useHistoryStore.getState().clear();
    set((state) => {
      const withoutDupe = state.recents.filter((p) => p.id !== project.id);
      return {
        currentProject: project,
        recents: [project, ...withoutDupe].slice(0, RECENTS_LIMIT),
      };
    });
    hydrateHistoryFor(project);
  },
  closeProject: () => {
    const prev = get().currentProject;
    if (prev) {
      persistHistoryFor(prev);
    }
    useHistoryStore.getState().clear();
    set({ currentProject: null });
  },
  addRecent: (project) => {
    set((state) => {
      const withoutDupe = state.recents.filter((p) => p.id !== project.id);
      return { recents: [project, ...withoutDupe].slice(0, RECENTS_LIMIT) };
    });
  },
  hydrateRecents: (projects) => set({ recents: projects.slice(0, RECENTS_LIMIT) }),
  clearRecents: () => set({ recents: [] }),
}));
