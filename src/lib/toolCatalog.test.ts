import { describe, expect, it } from "vitest";
import { CATALOG, getToolsFor, type TaskKind } from "@/lib/toolCatalog";

describe("toolCatalog", () => {
  it("provides tools for every TaskKind", () => {
    const expected: TaskKind[] = [
      "TextGeneration",
      "ImageGeneration",
      "ImageEdit",
      "Inpaint",
      "Upscale",
      "Logo",
      "TextToVideo",
      "ImageToVideo",
      "VideoMontage",
      "Text3D",
      "Image3D",
      "ImageAnalysis",
      "DepthMap",
    ];
    for (const task of expected) {
      const tools = getToolsFor(task);
      expect(tools.length).toBeGreaterThan(0);
      expect(tools.some((t) => t.tier === "primary")).toBe(true);
    }
    // Reference CATALOG to ensure the named export is reachable from consumers.
    expect(Object.keys(CATALOG).length).toBe(expected.length);
  });

  it("Logo has Ideogram as the only tool", () => {
    const tools = getToolsFor("Logo");
    expect(tools).toHaveLength(1);
    expect(tools[0]?.id).toBe("ideogram-v3");
    expect(tools[0]?.tier).toBe("primary");
  });

  it("TextToVideo lists Kling V2 primary, V1.5 + Runway + Higgsfield as fallbacks", () => {
    const tools = getToolsFor("TextToVideo");
    expect(tools[0]?.id).toBe("fal-kling-v2-master");
    expect(tools[0]?.tier).toBe("primary");
    expect(tools.some((t) => t.id === "fal-kling-v15" && t.tier === "fallback")).toBe(true);
    expect(tools.some((t) => t.id === "runway-gen3" && t.tier === "fallback")).toBe(true);
    expect(tools.some((t) => t.id === "higgsfield" && t.tier === "fallback")).toBe(true);
  });

  it("VideoMontage offers Shotstack primary + Remotion alternative", () => {
    const tools = getToolsFor("VideoMontage");
    expect(tools[0]?.id).toBe("shotstack");
    expect(tools.some((t) => t.id === "remotion" && t.tier === "alternative")).toBe(true);
  });
});
