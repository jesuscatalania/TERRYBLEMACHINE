import { create } from "zustand";

export type ShortcutScope =
  | "global"
  | "page"
  | "module:website"
  | "module:graphic2d"
  | "module:graphic3d"
  | "module:video"
  | "module:typography";

export interface ShortcutEntry {
  /** Stable id within (scope, combo) — used for unregister. */
  id: string;
  /** Canonical combo string, e.g. "Mod+Z", "Mod+Shift+Z", "Mod+1", "?". */
  combo: string;
  handler: () => void;
  scope: ShortcutScope;
  /** Human-readable description for the help overlay. */
  label: string;
  /** Optional gating predicate (e.g., disable when on /settings). */
  when?: () => boolean;
}

interface KeyboardState {
  entries: Map<string, ShortcutEntry>;
  register: (entry: ShortcutEntry) => void;
  unregister: (id: string) => void;
  list: () => ShortcutEntry[];
  /** All entries matching a combo, in registration order. */
  entriesByCombo: (combo: string) => ShortcutEntry[];
}

export const useKeyboardStore = create<KeyboardState>((set, get) => ({
  entries: new Map(),
  register: (entry) =>
    set((state) => {
      const next = new Map(state.entries);
      next.set(entry.id, entry);
      return { entries: next };
    }),
  unregister: (id) =>
    set((state) => {
      const next = new Map(state.entries);
      next.delete(id);
      return { entries: next };
    }),
  list: () => Array.from(get().entries.values()),
  entriesByCombo: (combo) => Array.from(get().entries.values()).filter((e) => e.combo === combo),
}));
