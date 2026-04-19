import { afterEach, describe, expect, it } from "vitest";
import { useKeyboardStore } from "@/stores/keyboardStore";

describe("keyboardStore", () => {
  afterEach(() => {
    useKeyboardStore.setState({ entries: new Map() });
  });

  it("register adds entry; list returns it", () => {
    const handler = () => {};
    useKeyboardStore.getState().register({
      id: "test:undo",
      combo: "Mod+Z",
      handler,
      scope: "global",
      label: "Undo",
    });
    expect(useKeyboardStore.getState().list()).toHaveLength(1);
    expect(useKeyboardStore.getState().list()[0]?.label).toBe("Undo");
  });

  it("unregister removes entry by id", () => {
    useKeyboardStore.getState().register({
      id: "x",
      combo: "Mod+S",
      handler: () => {},
      scope: "global",
      label: "Save",
    });
    useKeyboardStore.getState().unregister("x");
    expect(useKeyboardStore.getState().list()).toHaveLength(0);
  });

  it("multiple entries with same combo coexist (priority resolved by dispatcher, not store)", () => {
    useKeyboardStore.getState().register({
      id: "g",
      combo: "Mod+Enter",
      handler: () => {},
      scope: "global",
      label: "global",
    });
    useKeyboardStore.getState().register({
      id: "p",
      combo: "Mod+Enter",
      handler: () => {},
      scope: "page",
      label: "page",
    });
    expect(useKeyboardStore.getState().list()).toHaveLength(2);
  });

  it("entriesByCombo groups by canonical combo", () => {
    useKeyboardStore.getState().register({
      id: "a",
      combo: "Mod+Enter",
      handler: () => {},
      scope: "global",
      label: "a",
    });
    useKeyboardStore.getState().register({
      id: "b",
      combo: "Mod+Enter",
      handler: () => {},
      scope: "page",
      label: "b",
    });
    const grouped = useKeyboardStore.getState().entriesByCombo("Mod+Enter");
    expect(grouped).toHaveLength(2);
  });
});
