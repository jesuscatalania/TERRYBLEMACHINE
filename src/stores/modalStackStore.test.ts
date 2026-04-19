import { afterEach, describe, expect, it } from "vitest";
import { useModalStackStore } from "@/stores/modalStackStore";

afterEach(() => {
  useModalStackStore.setState({ stack: [] });
});

describe("modalStackStore", () => {
  it("starts empty — isAnyOpen=false", () => {
    expect(useModalStackStore.getState().isAnyOpen()).toBe(false);
    expect(useModalStackStore.getState().stack).toEqual([]);
  });

  it("push appends to stack and is idempotent per id", () => {
    useModalStackStore.getState().push("a");
    useModalStackStore.getState().push("a");
    expect(useModalStackStore.getState().stack).toEqual(["a"]);
    useModalStackStore.getState().push("b");
    expect(useModalStackStore.getState().stack).toEqual(["a", "b"]);
  });

  it("pop removes by id without affecting other ids", () => {
    useModalStackStore.getState().push("a");
    useModalStackStore.getState().push("b");
    useModalStackStore.getState().push("c");
    useModalStackStore.getState().pop("b");
    expect(useModalStackStore.getState().stack).toEqual(["a", "c"]);
  });

  it("pop on missing id is a no-op", () => {
    useModalStackStore.getState().push("a");
    useModalStackStore.getState().pop("zzz");
    expect(useModalStackStore.getState().stack).toEqual(["a"]);
  });

  it("isTop returns true only for the last-pushed id", () => {
    useModalStackStore.getState().push("a");
    useModalStackStore.getState().push("b");
    expect(useModalStackStore.getState().isTop("a")).toBe(false);
    expect(useModalStackStore.getState().isTop("b")).toBe(true);
  });

  it("isTop returns false when stack is empty", () => {
    expect(useModalStackStore.getState().isTop("anything")).toBe(false);
  });

  it("isAnyOpen reflects stack state across push/pop", () => {
    expect(useModalStackStore.getState().isAnyOpen()).toBe(false);
    useModalStackStore.getState().push("a");
    expect(useModalStackStore.getState().isAnyOpen()).toBe(true);
    useModalStackStore.getState().pop("a");
    expect(useModalStackStore.getState().isAnyOpen()).toBe(false);
  });

  it("after popping the top, the previous modal becomes the new top", () => {
    useModalStackStore.getState().push("a");
    useModalStackStore.getState().push("b");
    useModalStackStore.getState().pop("b");
    expect(useModalStackStore.getState().isTop("a")).toBe(true);
  });
});
