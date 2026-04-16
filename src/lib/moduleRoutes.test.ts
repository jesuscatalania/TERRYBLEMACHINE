import { describe, expect, it } from "vitest";
import { moduleToPath, pathToModule } from "@/lib/moduleRoutes";

describe("moduleRoutes", () => {
  it("maps moduleId → path", () => {
    expect(moduleToPath("website")).toBe("/website");
    expect(moduleToPath("graphic3d")).toBe("/graphic3d");
    expect(moduleToPath("typography")).toBe("/typography");
  });

  it("maps top-level module paths back to moduleId", () => {
    expect(pathToModule("/website")).toBe("website");
    expect(pathToModule("/video")).toBe("video");
  });

  it("tolerates trailing segments", () => {
    expect(pathToModule("/website/edit")).toBe("website");
  });

  it("returns undefined for non-module paths", () => {
    expect(pathToModule("/")).toBeUndefined();
    expect(pathToModule("/design-system")).toBeUndefined();
    expect(pathToModule("/unknown")).toBeUndefined();
  });
});
