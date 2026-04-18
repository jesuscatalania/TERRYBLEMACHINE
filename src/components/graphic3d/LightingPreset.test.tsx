import { describe, expect, it } from "vitest";
import { presetEnvFor } from "@/components/graphic3d/LightingPreset";

describe("presetEnvFor", () => {
  it("studio → studio environment", () => {
    expect(presetEnvFor("studio")).toBe("studio");
  });
  it("outdoor → sunset environment", () => {
    expect(presetEnvFor("outdoor")).toBe("sunset");
  });
  it("dramatic → night environment", () => {
    expect(presetEnvFor("dramatic")).toBe("night");
  });
});
