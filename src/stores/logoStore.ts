import { create } from "zustand";

interface LogoState {
  favorites: Set<string>;
  toggleFavorite: (url: string) => void;
  isFavorite: (url: string) => boolean;
  clearFavorites: () => void;
}

export const useLogoStore = create<LogoState>((set, get) => ({
  favorites: new Set<string>(),
  toggleFavorite: (url) =>
    set((state) => {
      const next = new Set(state.favorites);
      if (next.has(url)) next.delete(url);
      else next.add(url);
      return { favorites: next };
    }),
  isFavorite: (url) => get().favorites.has(url),
  clearFavorites: () => set({ favorites: new Set<string>() }),
}));
