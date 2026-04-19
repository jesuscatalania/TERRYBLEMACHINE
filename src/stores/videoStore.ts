import { create } from "zustand";
import type { VideoResult } from "@/lib/videoCommands";

export type SegmentKind = "ai" | "remotion" | "shotstack";

export interface Segment {
  id: string;
  kind: SegmentKind;
  label: string;
  duration_s: number;
  /** Remote URL from AI provider or Shotstack. */
  video_url?: string;
  /** Local cache path for AI-generated or Remotion-rendered clips. */
  local_path?: string | null;
  /** Provider model string for AI segments. */
  model?: string;
  /** Busy flag while generating. */
  busy?: boolean;
  /** Error after a failed generation. */
  error?: string;
}

interface VideoState {
  segments: Segment[];
  addSegment: (s: Omit<Segment, "id">) => string;
  updateSegment: (id: string, patch: Partial<Segment>) => void;
  removeSegment: (id: string) => void;
  moveSegment: (fromIndex: number, toIndex: number) => void;
  applyVideoResult: (id: string, r: VideoResult) => void;
  reset: () => void;
}

export const VIDEO_SEGMENTS_STORAGE_KEY = "tm:video:segments";

let idCounter = 0;
const nextId = () => `seg-${Date.now()}-${++idCounter}`;

function loadInitial(): Segment[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(VIDEO_SEGMENTS_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as Segment[];
    if (!Array.isArray(parsed)) return [];
    // Clear transient busy flags — a segment can't remain mid-render
    // across a page reload. Errors stay so the user can retry.
    return parsed.map((s) => ({ ...s, busy: false }));
  } catch {
    return [];
  }
}

function persist(segments: Segment[]) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(VIDEO_SEGMENTS_STORAGE_KEY, JSON.stringify(segments));
  } catch {
    // localStorage full or disabled — silently ignore
  }
}

export const useVideoStore = create<VideoState>((set) => ({
  segments: loadInitial(),
  addSegment: (s) => {
    const id = nextId();
    set((state) => {
      const next = [...state.segments, { id, ...s }];
      persist(next);
      return { segments: next };
    });
    return id;
  },
  updateSegment: (id, patch) =>
    set((state) => {
      const next = state.segments.map((s) => (s.id === id ? { ...s, ...patch } : s));
      persist(next);
      return { segments: next };
    }),
  removeSegment: (id) =>
    set((state) => {
      const next = state.segments.filter((s) => s.id !== id);
      persist(next);
      return { segments: next };
    }),
  moveSegment: (from, to) =>
    set((state) => {
      const next = [...state.segments];
      if (from < 0 || from >= next.length || to < 0 || to >= next.length) return state;
      const [moved] = next.splice(from, 1);
      if (!moved) return state;
      next.splice(to, 0, moved);
      persist(next);
      return { segments: next };
    }),
  applyVideoResult: (id, r) =>
    set((state) => {
      const next = state.segments.map((s) =>
        s.id === id
          ? {
              ...s,
              busy: false,
              error: undefined,
              video_url: r.video_url,
              local_path: r.local_path,
              model: r.model,
              duration_s: r.duration_s ?? s.duration_s,
            }
          : s,
      );
      persist(next);
      return { segments: next };
    }),
  reset: () => {
    persist([]);
    set({ segments: [] });
  },
}));
