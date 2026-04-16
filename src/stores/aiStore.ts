import { create } from "zustand";

export type Provider =
  | "claude"
  | "kling"
  | "runway"
  | "higgsfield"
  | "shotstack"
  | "ideogram"
  | "meshy"
  | "fal"
  | "replicate";

export interface Budget {
  usedCents: number;
  limitCents: number;
  periodStartedAt: string;
}

export interface CacheStats {
  hits: number;
  misses: number;
  size: number;
}

export interface ActiveRequest {
  id: string;
  provider: Provider;
  task: string;
  startedAt?: string;
}

export type BudgetStatus = "ok" | "warn" | "block";

const WARN_RATIO = 0.8;

export interface AiState {
  budget: Budget;
  cache: CacheStats;
  activeRequests: ActiveRequest[];
  recordSpend: (cents: number) => void;
  setBudgetLimit: (cents: number) => void;
  getBudgetStatus: () => BudgetStatus;
  recordCacheHit: () => void;
  recordCacheMiss: () => void;
  setCacheSize: (size: number) => void;
  startRequest: (request: ActiveRequest) => void;
  finishRequest: (id: string) => void;
}

export const useAiStore = create<AiState>((set, get) => ({
  budget: {
    usedCents: 0,
    limitCents: 5000,
    periodStartedAt: new Date().toISOString(),
  },
  cache: { hits: 0, misses: 0, size: 0 },
  activeRequests: [],

  recordSpend: (cents) =>
    set((state) => ({
      budget: { ...state.budget, usedCents: state.budget.usedCents + cents },
    })),

  setBudgetLimit: (limitCents) => set((state) => ({ budget: { ...state.budget, limitCents } })),

  getBudgetStatus: () => {
    const { usedCents, limitCents } = get().budget;
    if (limitCents <= 0) return "ok";
    const ratio = usedCents / limitCents;
    if (ratio >= 1) return "block";
    if (ratio >= WARN_RATIO) return "warn";
    return "ok";
  },

  recordCacheHit: () => set((state) => ({ cache: { ...state.cache, hits: state.cache.hits + 1 } })),

  recordCacheMiss: () =>
    set((state) => ({ cache: { ...state.cache, misses: state.cache.misses + 1 } })),

  setCacheSize: (size) => set((state) => ({ cache: { ...state.cache, size } })),

  startRequest: (request) =>
    set((state) => ({
      activeRequests: [
        ...state.activeRequests,
        { ...request, startedAt: request.startedAt ?? new Date().toISOString() },
      ],
    })),

  finishRequest: (id) =>
    set((state) => ({
      activeRequests: state.activeRequests.filter((r) => r.id !== id),
    })),
}));
