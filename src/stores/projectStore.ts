import { create } from "zustand";
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

export const useProjectStore = create<ProjectState>((set) => ({
  currentProject: null,
  recents: [],
  openProject: (project) => {
    // History is scoped per-project: wipe it on every project switch.
    useHistoryStore.getState().clear();
    set((state) => {
      const withoutDupe = state.recents.filter((p) => p.id !== project.id);
      return {
        currentProject: project,
        recents: [project, ...withoutDupe].slice(0, RECENTS_LIMIT),
      };
    });
  },
  closeProject: () => {
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
