import { describe, expect, it } from "vitest";
import { eventToCombo } from "@/lib/canonicalCombo";

function ev(opts: Partial<KeyboardEventInit> & { key: string }) {
  return new KeyboardEvent("keydown", opts);
}

describe("eventToCombo", () => {
  it("plain letter", () => expect(eventToCombo(ev({ key: "a" }))).toBe("A"));
  it("Mod+Z (mac meta)", () => expect(eventToCombo(ev({ key: "z", metaKey: true }))).toBe("Mod+Z"));
  it("Mod+Z (linux/win ctrl)", () =>
    expect(eventToCombo(ev({ key: "z", ctrlKey: true }))).toBe("Mod+Z"));
  it("Mod+Shift+Z", () =>
    expect(eventToCombo(ev({ key: "z", metaKey: true, shiftKey: true }))).toBe("Mod+Shift+Z"));
  it("Mod+/ uses literal slash", () =>
    expect(eventToCombo(ev({ key: "/", metaKey: true }))).toBe("Mod+/"));
  it("? maps to ?", () => expect(eventToCombo(ev({ key: "?" }))).toBe("?"));
  it("Mod+Enter", () =>
    expect(eventToCombo(ev({ key: "Enter", metaKey: true }))).toBe("Mod+Enter"));
  it("Mod+1", () => expect(eventToCombo(ev({ key: "1", metaKey: true }))).toBe("Mod+1"));
});
