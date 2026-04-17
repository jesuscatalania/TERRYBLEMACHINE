import { beforeEach, describe, expect, it } from "vitest";
import { useAppStore } from "@/stores/appStore";
import { useHistoryStore } from "@/stores/historyStore";

describe("store integration — appStore ↔ historyStore", () => {
  beforeEach(() => {
    useAppStore.setState({
      theme: "dark",
      sidebarOpen: true,
      activeModule: "website",
    });
    useHistoryStore.setState({ past: [], future: [] });
  });

  it("setActiveModule pushes an undoable command onto history", () => {
    useAppStore.getState().setActiveModule("video");
    const past = useHistoryStore.getState().past;
    expect(past).toHaveLength(1);
    expect(past[0]?.label).toBe("Switch to video");
    expect(useAppStore.getState().activeModule).toBe("video");
  });

  it("undo reverts setActiveModule to the previous module", () => {
    useAppStore.getState().setActiveModule("video");
    const reverted = useHistoryStore.getState().undo();
    expect(reverted).toBe(true);
    expect(useAppStore.getState().activeModule).toBe("website");
    expect(useHistoryStore.getState().past).toHaveLength(0);
    expect(useHistoryStore.getState().future).toHaveLength(1);
  });

  it("redo re-applies setActiveModule", () => {
    useAppStore.getState().setActiveModule("video");
    useHistoryStore.getState().undo();
    const replayed = useHistoryStore.getState().redo();
    expect(replayed).toBe(true);
    expect(useAppStore.getState().activeModule).toBe("video");
    expect(useHistoryStore.getState().past).toHaveLength(1);
    expect(useHistoryStore.getState().future).toHaveLength(0);
  });

  it("setActiveModule is a no-op when the target equals the current module", () => {
    useAppStore.getState().setActiveModule("website");
    expect(useHistoryStore.getState().past).toHaveLength(0);
  });
});

describe("historyStore — serialize / hydrate", () => {
  beforeEach(() => {
    useHistoryStore.setState({ past: [], future: [] });
  });

  it("serialize + hydrate round-trips labels across both stacks", () => {
    const history = useHistoryStore.getState();
    history.push({ label: "first", do: () => {}, undo: () => {} });
    history.push({ label: "second", do: () => {}, undo: () => {} });
    history.undo(); // moves "second" onto future

    const raw = useHistoryStore.getState().serialize();

    // Wipe and rehydrate from the serialised payload.
    useHistoryStore.setState({ past: [], future: [] });
    useHistoryStore.getState().hydrate(raw);

    const { past, future } = useHistoryStore.getState();
    expect(past.map((c) => c.label)).toEqual(["first"]);
    expect(future.map((c) => c.label)).toEqual(["second"]);
  });

  it("hydrate leaves stacks untouched on malformed JSON", () => {
    useHistoryStore.getState().push({ label: "keep me", do: () => {}, undo: () => {} });
    useHistoryStore.getState().hydrate("{not json");
    expect(useHistoryStore.getState().past.map((c) => c.label)).toEqual(["keep me"]);
  });

  it("hydrated commands are inert — undo/redo do not crash or mutate external state", () => {
    const raw = JSON.stringify({
      past: [{ label: "orphan", timestamp: "2026-04-17T00:00:00Z" }],
      future: [],
    });
    useHistoryStore.getState().hydrate(raw);
    // Undo the marker — no original closure, so it's a no-op. Should still
    // succeed (no throw) and move the entry to future.
    expect(useHistoryStore.getState().undo()).toBe(true);
    expect(useHistoryStore.getState().future).toHaveLength(1);
  });
});
