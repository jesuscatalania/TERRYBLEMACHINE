import { beforeEach, describe, expect, it, vi } from "vitest";
import { type CommandInput, HISTORY_LIMIT, useHistoryStore } from "@/stores/historyStore";

function makeCommand(overrides: Partial<CommandInput> = {}): CommandInput {
  return {
    label: "noop",
    do: vi.fn(),
    undo: vi.fn(),
    ...overrides,
  };
}

describe("historyStore", () => {
  beforeEach(() => {
    useHistoryStore.setState({ past: [], future: [] });
  });

  it("starts empty — canUndo/canRedo both false", () => {
    const s = useHistoryStore.getState();
    expect(s.canUndo()).toBe(false);
    expect(s.canRedo()).toBe(false);
  });

  it("push() executes the command and records it in past", () => {
    const doFn = vi.fn();
    useHistoryStore.getState().push(makeCommand({ do: doFn, label: "a" }));
    expect(doFn).toHaveBeenCalledOnce();
    expect(useHistoryStore.getState().past).toHaveLength(1);
    expect(useHistoryStore.getState().past[0]?.label).toBe("a");
  });

  it("push() clears any existing future", () => {
    useHistoryStore.getState().push(makeCommand({ label: "a" }));
    useHistoryStore.getState().undo();
    expect(useHistoryStore.getState().future).toHaveLength(1);
    useHistoryStore.getState().push(makeCommand({ label: "b" }));
    expect(useHistoryStore.getState().future).toHaveLength(0);
  });

  it("undo() runs the latest command's undo and moves it to future", () => {
    const undoFn = vi.fn();
    useHistoryStore.getState().push(makeCommand({ label: "a", undo: undoFn }));
    const result = useHistoryStore.getState().undo();
    expect(result).toBe(true);
    expect(undoFn).toHaveBeenCalledOnce();
    expect(useHistoryStore.getState().past).toHaveLength(0);
    expect(useHistoryStore.getState().future).toHaveLength(1);
  });

  it("undo() on empty past returns false and does nothing", () => {
    const result = useHistoryStore.getState().undo();
    expect(result).toBe(false);
  });

  it("redo() re-runs the latest future command's do and moves it to past", () => {
    const doFn = vi.fn();
    useHistoryStore.getState().push(makeCommand({ label: "a", do: doFn }));
    useHistoryStore.getState().undo();
    doFn.mockClear();
    const result = useHistoryStore.getState().redo();
    expect(result).toBe(true);
    expect(doFn).toHaveBeenCalledOnce();
    expect(useHistoryStore.getState().past).toHaveLength(1);
    expect(useHistoryStore.getState().future).toHaveLength(0);
  });

  it("redo() on empty future returns false", () => {
    expect(useHistoryStore.getState().redo()).toBe(false);
  });

  it("canUndo/canRedo reflect stack state", () => {
    const s = useHistoryStore.getState();
    s.push(makeCommand({ label: "a" }));
    expect(s.canUndo()).toBe(true);
    s.undo();
    expect(s.canUndo()).toBe(false);
    expect(s.canRedo()).toBe(true);
  });

  it(`caps past at ${HISTORY_LIMIT} entries (oldest dropped)`, () => {
    for (let i = 0; i < HISTORY_LIMIT + 5; i++) {
      useHistoryStore.getState().push(makeCommand({ label: `c${i}` }));
    }
    const past = useHistoryStore.getState().past;
    expect(past).toHaveLength(HISTORY_LIMIT);
    // oldest retained is c5 (c0..c4 dropped)
    expect(past[0]?.label).toBe("c5");
  });

  it("clear() empties both stacks", () => {
    useHistoryStore.getState().push(makeCommand({ label: "a" }));
    useHistoryStore.getState().undo();
    useHistoryStore.getState().clear();
    const s = useHistoryStore.getState();
    expect(s.past).toHaveLength(0);
    expect(s.future).toHaveLength(0);
  });

  it("each pushed command carries a unique id + timestamp", () => {
    useHistoryStore.getState().push(makeCommand({ label: "a" }));
    useHistoryStore.getState().push(makeCommand({ label: "b" }));
    const [first, second] = useHistoryStore.getState().past;
    expect(first?.id).toBeDefined();
    expect(second?.id).toBeDefined();
    expect(first?.id).not.toBe(second?.id);
    expect(first?.timestamp).toBeDefined();
  });
});
