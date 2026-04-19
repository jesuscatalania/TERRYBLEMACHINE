import { afterEach, describe, expect, it } from "vitest";
import { CHAT_STORAGE_KEY, useChatStore } from "@/stores/chatStore";

describe("chatStore", () => {
  afterEach(() => {
    useChatStore.getState().newChat();
    window.localStorage.removeItem(CHAT_STORAGE_KEY);
  });

  it("starts empty", () => {
    expect(useChatStore.getState().messages).toEqual([]);
  });

  it("addMessage appends with id + role", () => {
    useChatStore.getState().addMessage({ role: "user", content: "hi" });
    const msgs = useChatStore.getState().messages;
    expect(msgs).toHaveLength(1);
    expect(msgs[0]?.role).toBe("user");
    expect(msgs[0]?.content).toBe("hi");
    expect(msgs[0]?.id).toBeTruthy();
  });

  it("appendChunk concatenates onto a streaming assistant message", () => {
    useChatStore.getState().addMessage({ role: "assistant", content: "" });
    const id = useChatStore.getState().messages[0]?.id;
    if (!id) throw new Error("id must exist");
    useChatStore.getState().appendChunk(id, "Hello ");
    useChatStore.getState().appendChunk(id, "world");
    expect(useChatStore.getState().messages[0]?.content).toBe("Hello world");
  });

  it("newChat clears messages", () => {
    useChatStore.getState().addMessage({ role: "user", content: "hi" });
    useChatStore.getState().newChat();
    expect(useChatStore.getState().messages).toEqual([]);
  });

  it("persists to localStorage", () => {
    useChatStore.getState().addMessage({ role: "user", content: "persisted" });
    const raw = window.localStorage.getItem(CHAT_STORAGE_KEY);
    expect(raw).toBeTruthy();
    expect(raw).toContain("persisted");
  });
});
