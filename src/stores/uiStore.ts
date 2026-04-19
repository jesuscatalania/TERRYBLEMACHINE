import { create } from "zustand";

export type NotificationKind = "success" | "error" | "warning" | "info";

export interface Modal {
  id: string;
  title: string;
  body?: string;
  dismissible?: boolean;
}

export interface Notification {
  id: string;
  kind: NotificationKind;
  message: string;
  detail?: string;
  createdAt: string;
  /** Optional progress for long-running operations: { current, total }. */
  progress?: { current: number; total: number };
}

export type NotificationInput = Omit<Notification, "id" | "createdAt"> &
  Partial<Pick<Notification, "id" | "createdAt">>;

export interface UiState {
  modals: Modal[];
  notifications: Notification[];
  loadingJobs: number;
  openModal: (modal: Modal) => void;
  closeModal: (id: string) => void;
  notify: (input: NotificationInput) => string;
  dismissNotification: (id: string) => void;
  updateNotification: (id: string, patch: Partial<Notification>) => void;
  startLoading: () => void;
  finishLoading: () => void;
  isLoading: () => boolean;
}

const makeId = () =>
  typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `n_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

export const useUiStore = create<UiState>((set, get) => ({
  modals: [],
  notifications: [],
  loadingJobs: 0,

  openModal: (modal) =>
    set((state) =>
      state.modals.some((m) => m.id === modal.id) ? state : { modals: [...state.modals, modal] },
    ),

  closeModal: (id) => set((state) => ({ modals: state.modals.filter((m) => m.id !== id) })),

  notify: (input) => {
    const id = input.id ?? makeId();
    const createdAt = input.createdAt ?? new Date().toISOString();
    set((state) => ({
      notifications: [...state.notifications, { ...input, id, createdAt }],
    }));
    return id;
  },

  dismissNotification: (id) =>
    set((state) => ({
      notifications: state.notifications.filter((n) => n.id !== id),
    })),

  updateNotification: (id, patch) =>
    set((state) => ({
      notifications: state.notifications.map((n) => (n.id === id ? { ...n, ...patch } : n)),
    })),

  startLoading: () => set((state) => ({ loadingJobs: state.loadingJobs + 1 })),

  finishLoading: () => set((state) => ({ loadingJobs: Math.max(0, state.loadingJobs - 1) })),

  isLoading: () => get().loadingJobs > 0,
}));
