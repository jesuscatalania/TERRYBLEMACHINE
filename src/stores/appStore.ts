import { create } from "zustand";

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

export const useAppStore = create<AppState>((set) => ({
  theme: "dark",
  sidebarOpen: true,
  activeModule: "website",
  setTheme: (theme) => set({ theme }),
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),
  setActiveModule: (activeModule) => set({ activeModule }),
}));
