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

export interface HistoryState {
  past: Command[];
  future: Command[];
  push: (command: CommandInput) => void;
  undo: () => boolean;
  redo: () => boolean;
  canUndo: () => boolean;
  canRedo: () => boolean;
  clear: () => void;
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
}));
