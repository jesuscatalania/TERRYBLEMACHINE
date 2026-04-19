import { create } from "zustand";

interface ModalStackState {
  /** ids of currently-open modals, top of stack last */
  stack: string[];
  push: (id: string) => void;
  pop: (id: string) => void;
  isTop: (id: string) => boolean;
  isAnyOpen: () => boolean;
}

/**
 * Modal-stack manager so only the top modal reacts to Escape, and so the
 * global keyboard dispatcher can suppress page/module shortcuts while any
 * modal is open. Ids are opaque and supplied by the caller (Modal uses
 * React's `useId()`).
 */
export const useModalStackStore = create<ModalStackState>((set, get) => ({
  stack: [],
  push: (id) =>
    set((state) => (state.stack.includes(id) ? state : { stack: [...state.stack, id] })),
  pop: (id) => set((state) => ({ stack: state.stack.filter((x) => x !== id) })),
  isTop: (id) => {
    const stack = get().stack;
    return stack[stack.length - 1] === id;
  },
  isAnyOpen: () => get().stack.length > 0,
}));
