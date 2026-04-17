import { create } from "zustand";
import { useHistoryStore } from "@/stores/historyStore";

export type Theme = "dark" | "light";
export type ModuleId = "website" | "graphic2d" | "graphic3d" | "video" | "typography";

export interface AppState {
  theme: Theme;
  sidebarOpen: boolean;
  activeModule: ModuleId;
  setTheme: (theme: Theme) => void;
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
  setActiveModule: (module: ModuleId) => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  theme: "dark",
  sidebarOpen: true,
  activeModule: "website",
  setTheme: (theme) => set({ theme }),
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),
  /**
   * Routes module switches through the history store so they become
   * undoable. No-op when the module is unchanged to avoid dead stack entries.
   *
   * `historyStore.push` invokes `do()` eagerly, so the state transition
   * happens inside the pushed command — no extra `set` call needed here.
   */
  setActiveModule: (next) => {
    const prev = get().activeModule;
    if (prev === next) return;
    useHistoryStore.getState().push({
      label: `Switch to ${next}`,
      do: () => set({ activeModule: next }),
      undo: () => set({ activeModule: prev }),
    });
  },
}));
