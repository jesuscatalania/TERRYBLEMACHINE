import { create } from "zustand";

export const CHAT_STORAGE_KEY = "tm:chat:messages";

export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  createdAt: string;
}

interface ChatState {
  messages: ChatMessage[];
  addMessage: (m: { role: "user" | "assistant"; content: string }) => string;
  appendChunk: (id: string, chunk: string) => void;
  newChat: () => void;
  hydrate: () => void;
}

const makeId = () =>
  typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `m_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

function loadInitial(): ChatMessage[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(CHAT_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as ChatMessage[];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function persist(messages: ChatMessage[]) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(CHAT_STORAGE_KEY, JSON.stringify(messages));
  } catch {
    // localStorage full or disabled — silently ignore
  }
}

export const useChatStore = create<ChatState>((set) => ({
  messages: loadInitial(),
  addMessage: ({ role, content }) => {
    const msg: ChatMessage = {
      id: makeId(),
      role,
      content,
      createdAt: new Date().toISOString(),
    };
    set((state) => {
      const next = [...state.messages, msg];
      persist(next);
      return { messages: next };
    });
    return msg.id;
  },
  appendChunk: (id, chunk) =>
    set((state) => {
      const next = state.messages.map((m) =>
        m.id === id ? { ...m, content: m.content + chunk } : m,
      );
      persist(next);
      return { messages: next };
    }),
  newChat: () => {
    persist([]);
    set({ messages: [] });
  },
  hydrate: () => set({ messages: loadInitial() }),
}));
