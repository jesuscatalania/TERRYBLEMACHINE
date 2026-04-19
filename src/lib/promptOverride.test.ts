import { describe, expect, it } from "vitest";
import { OVERRIDE_ALIASES, parseOverride, resolveOverrideToModel } from "@/lib/promptOverride";

describe("parseOverride", () => {
  it("recognizes /slug at start", () => {
    const r = parseOverride("/flux a sunset");
    expect(r.override).toBe("flux");
    expect(r.cleanPrompt).toBe("a sunset");
    expect(r.slugLocation).toBe("start");
  });

  it("recognizes /slug at end", () => {
    const r = parseOverride("a sunset /flux");
    expect(r.override).toBe("flux");
    expect(r.cleanPrompt).toBe("a sunset");
    expect(r.slugLocation).toBe("end");
  });

  it("ignores /slug in the middle", () => {
    const r = parseOverride("a /flux sunset");
    expect(r.override).toBeUndefined();
    expect(r.cleanPrompt).toBe("a /flux sunset");
  });

  it("returns no override on plain prompt", () => {
    const r = parseOverride("a sunset over berlin");
    expect(r.override).toBeUndefined();
    expect(r.cleanPrompt).toBe("a sunset over berlin");
  });

  it("only matches known slugs (whitelist)", () => {
    const r = parseOverride("/unknown-slug-xyz a sunset");
    expect(r.override).toBeUndefined();
    expect(r.cleanPrompt).toBe("/unknown-slug-xyz a sunset");
  });

  it("first-position wins when both ends have a slug", () => {
    const r = parseOverride("/flux some prompt /sdxl");
    expect(r.override).toBe("flux");
    expect(r.cleanPrompt).toBe("some prompt /sdxl");
  });

  it("is case-insensitive on slugs", () => {
    const r = parseOverride("/Flux a sunset");
    expect(r.override).toBe("flux");
  });

  it("trims surrounding whitespace from cleanPrompt", () => {
    const r = parseOverride("  /flux  a sunset  ");
    expect(r.override).toBe("flux");
    expect(r.cleanPrompt).toBe("a sunset");
  });

  it("an empty prompt with only a slug yields the slug + empty cleanPrompt", () => {
    const r = parseOverride("/flux");
    expect(r.override).toBe("flux");
    expect(r.cleanPrompt).toBe("");
  });

  it("/kling alias resolves to FalKlingV2Master", () => {
    expect(resolveOverrideToModel("kling")).toBe("FalKlingV2Master");
  });

  it("/kling-v15 resolves to FalKlingV15", () => {
    expect(resolveOverrideToModel("kling-v15")).toBe("FalKlingV15");
  });

  it("unknown alias resolves to undefined", () => {
    expect(resolveOverrideToModel("xyz")).toBeUndefined();
    // Reference OVERRIDE_ALIASES so the named export path is exercised.
    expect(Object.keys(OVERRIDE_ALIASES).length).toBeGreaterThan(0);
  });
});
