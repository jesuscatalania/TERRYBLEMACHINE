import { create } from "zustand";

export const HISTORY_LIMIT = 50;

/** What a caller passes to `push()`. */
export interface CommandInput {
  /** Short human-readable description (used in history listings). */
  label: string;
  /** Applies the change. Called once on `push`, and again on `redo`. */
  do: () => void;
  /** Reverts the change. Called on `undo`. */
  undo: () => void;
}

/** Recorded command with id + timestamp. Lives on the stacks. */
export interface Command extends CommandInput {
  id: string;
  timestamp: string;
}

/** Shape written to disk — functions are stripped. */
export interface SerializableCommand {
  label: string;
  timestamp: string;
}

export interface HistoryState {
  past: Command[];
  future: Command[];
  push: (command: CommandInput) => void;
  undo: () => boolean;
  redo: () => boolean;
  canUndo: () => boolean;
  canRedo: () => boolean;
  clear: () => void;
  /**
   * Serialises the history stacks to a JSON string — labels + timestamps only.
   * Safe to write to disk (no functions, no closures).
   */
  serialize: () => string;
  /**
   * Rehydrates history stacks from a serialised JSON string.
   *
   * IMPORTANT: Hydrated commands are *read-only markers* — their `do` / `undo`
   * are no-ops because the original closures cannot be serialised. Calling
   * `undo()` / `redo()` on a hydrated command will not actually replay the
   * mutation; true replay requires a command registry, which is out of scope
   * for Phase 0 persistence.
   *
   * On malformed input, stacks are left untouched.
   */
  hydrate: (raw: string) => void;
}

const makeId = () =>
  typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `cmd_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

export const useHistoryStore = create<HistoryState>((set, get) => ({
  past: [],
  future: [],

  push: (input) => {
    input.do();
    set((state) => {
      const cmd: Command = {
        ...input,
        id: makeId(),
        timestamp: new Date().toISOString(),
      };
      const nextPast = [...state.past, cmd];
      // Drop oldest if over limit.
      if (nextPast.length > HISTORY_LIMIT) {
        nextPast.splice(0, nextPast.length - HISTORY_LIMIT);
      }
      return { past: nextPast, future: [] };
    });
  },

  undo: () => {
    const { past } = get();
    if (past.length === 0) return false;
    const cmd = past[past.length - 1];
    if (!cmd) return false;
    cmd.undo();
    set((state) => ({
      past: state.past.slice(0, -1),
      future: [cmd, ...state.future],
    }));
    return true;
  },

  redo: () => {
    const { future } = get();
    if (future.length === 0) return false;
    const cmd = future[0];
    if (!cmd) return false;
    cmd.do();
    set((state) => ({
      past: [...state.past, cmd],
      future: state.future.slice(1),
    }));
    return true;
  },

  canUndo: () => get().past.length > 0,
  canRedo: () => get().future.length > 0,
  clear: () => set({ past: [], future: [] }),

  serialize: () => {
    const { past, future } = get();
    const toMarker = (c: Command): SerializableCommand => ({
      label: c.label,
      timestamp: c.timestamp,
    });
    return JSON.stringify({
      past: past.map(toMarker),
      future: future.map(toMarker),
    });
  },

  hydrate: (raw) => {
    try {
      const parsed = JSON.parse(raw) as {
        past?: SerializableCommand[];
        future?: SerializableCommand[];
      };
      if (!parsed || typeof parsed !== "object") return;
      const noop = () => {};
      const fromMarker = (m: SerializableCommand): Command => ({
        label: m.label,
        timestamp: m.timestamp,
        id: makeId(),
        do: noop,
        undo: noop,
      });
      set({
        past: Array.isArray(parsed.past) ? parsed.past.map(fromMarker) : [],
        future: Array.isArray(parsed.future) ? parsed.future.map(fromMarker) : [],
      });
    } catch {
      // Corrupt file → leave stacks untouched.
    }
  },
}));
