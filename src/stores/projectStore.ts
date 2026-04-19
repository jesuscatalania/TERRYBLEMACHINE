import { create } from "zustand";
import { formatError } from "@/lib/formatError";
import { readProjectHistory, writeProjectHistory } from "@/lib/projectCommands";
import type { ModuleId } from "@/stores/appStore";
import { useHistoryStore } from "@/stores/historyStore";
import { useUiStore } from "@/stores/uiStore";

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

    // Hydrate is async while openProject is sync — guard against two races:
    //   1) User switches to a different project before hydrate resolves.
    //   2) User performs an undoable action before hydrate resolves, which
    //      would otherwise get clobbered by the disk state.
    // Only apply the hydrated stacks if the same project is still open AND
    // nothing has been pushed onto the live history in the interim.
    const targetId = project.id;
    readProjectHistory(project.path)
      .then((raw) => {
        const current = get().currentProject;
        const hs = useHistoryStore.getState();
        if (current?.id === targetId && hs.past.length === 0 && hs.future.length === 0) {
          hs.hydrate(raw);
        }
      })
      .catch((err) => {
        useUiStore.getState().notify({
          kind: "warning",
          message: "Undo-Verlauf konnte nicht geladen werden",
          detail: formatError(err),
        });
      });
  },
  closeProject: () => {
    const prev = get().currentProject;
    if (prev) {
      // Fire-and-forget persistence, but surface write failures so the user
      // knows their undo history didn't make it to disk.
      const json = useHistoryStore.getState().serialize();
      writeProjectHistory(prev.path, json).catch((err) => {
        useUiStore.getState().notify({
          kind: "warning",
          message: "Undo-Verlauf konnte nicht gespeichert werden",
          detail: formatError(err),
        });
      });
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
