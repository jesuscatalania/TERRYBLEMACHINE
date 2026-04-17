import { beforeEach, describe, expect, it } from "vitest";
import { PROMPT_HISTORY_LIMIT, usePromptHistoryStore } from "@/stores/promptHistoryStore";

describe("promptHistoryStore", () => {
  beforeEach(() => {
    usePromptHistoryStore.setState({ entries: [] });
  });

  it("is empty by default", () => {
    expect(usePromptHistoryStore.getState().entries).toEqual([]);
  });

  it("push() prepends new entries newest-first", () => {
    usePromptHistoryStore.getState().push("first");
    usePromptHistoryStore.getState().push("second");
    expect(usePromptHistoryStore.getState().entries.map((e) => e.text)).toEqual([
      "second",
      "first",
    ]);
  });

  it("push() ignores empty and whitespace-only input", () => {
    usePromptHistoryStore.getState().push("   ");
    usePromptHistoryStore.getState().push("\n\t");
    expect(usePromptHistoryStore.getState().entries).toHaveLength(0);
  });

  it("push() dedupes consecutive identical entries", () => {
    usePromptHistoryStore.getState().push("same");
    usePromptHistoryStore.getState().push("same");
    expect(usePromptHistoryStore.getState().entries).toHaveLength(1);
  });

  it("push() moves an existing entry to the top on reuse", () => {
    const s = usePromptHistoryStore.getState();
    s.push("alpha");
    s.push("beta");
    s.push("alpha");
    expect(usePromptHistoryStore.getState().entries.map((e) => e.text)).toEqual(["alpha", "beta"]);
  });

  it(`caps history at ${PROMPT_HISTORY_LIMIT} entries`, () => {
    for (let i = 0; i < PROMPT_HISTORY_LIMIT + 5; i++) {
      usePromptHistoryStore.getState().push(`prompt-${i}`);
    }
    expect(usePromptHistoryStore.getState().entries).toHaveLength(PROMPT_HISTORY_LIMIT);
  });

  it("clear() empties the list", () => {
    usePromptHistoryStore.getState().push("a");
    usePromptHistoryStore.getState().push("b");
    usePromptHistoryStore.getState().clear();
    expect(usePromptHistoryStore.getState().entries).toEqual([]);
  });

  it("each entry has an id and createdAt", () => {
    usePromptHistoryStore.getState().push("hello");
    const entry = usePromptHistoryStore.getState().entries[0];
    expect(entry?.id).toBeDefined();
    expect(entry?.createdAt).toBeDefined();
    expect(entry?.text).toBe("hello");
  });
});
