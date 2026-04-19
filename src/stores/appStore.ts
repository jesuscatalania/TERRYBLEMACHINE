import { create } from "zustand";
import { useHistoryStore } from "@/stores/historyStore";

export type Theme = "dark" | "light";
export type ModuleId = "website" | "graphic2d" | "graphic3d" | "video" | "typography";

export interface AppState {
  theme: Theme;
  sidebarOpen: boolean;
  activeModule: ModuleId;
  /**
   * Generate handler registered by the currently-mounted module page.
   * The header's global Generate button calls this. Pages MUST register
   * on mount via `setActiveGenerate(submit)` and clear on unmount via
   * `setActiveGenerate(null)` — otherwise the header button is a no-op.
   */
  activeGenerate: (() => void) | null;
  setTheme: (theme: Theme) => void;
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
  setActiveModule: (module: ModuleId) => void;
  setActiveGenerate: (handler: (() => void) | null) => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  theme: "dark",
  sidebarOpen: true,
  activeModule: "website",
  activeGenerate: null,
  setTheme: (theme) => set({ theme }),
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),
  setActiveGenerate: (handler) => set({ activeGenerate: handler }),
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
