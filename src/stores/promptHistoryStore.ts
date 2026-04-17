import { create } from "zustand";

export const PROMPT_HISTORY_LIMIT = 20;

export interface PromptHistoryEntry {
  id: string;
  text: string;
  createdAt: string;
}

export interface PromptHistoryState {
  entries: PromptHistoryEntry[];
  /** Push a prompt. Ignores empty input. Dedupes and re-promotes existing matches. */
  push: (text: string) => void;
  clear: () => void;
}

const makeId = () =>
  typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `h_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

export const usePromptHistoryStore = create<PromptHistoryState>((set) => ({
  entries: [],
  push: (text) => {
    const trimmed = text.trim();
    if (!trimmed) return;
    set((state) => {
      const withoutDupe = state.entries.filter((e) => e.text !== trimmed);
      const entry: PromptHistoryEntry = {
        id: makeId(),
        text: trimmed,
        createdAt: new Date().toISOString(),
      };
      return { entries: [entry, ...withoutDupe].slice(0, PROMPT_HISTORY_LIMIT) };
    });
  },
  clear: () => set({ entries: [] }),
}));
