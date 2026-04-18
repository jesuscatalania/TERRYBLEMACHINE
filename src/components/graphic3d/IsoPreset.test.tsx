import { describe, expect, it } from "vitest";
import { cameraForIso } from "@/components/graphic3d/IsoPreset";

describe("cameraForIso", () => {
  it("none returns null", () => {
    expect(cameraForIso("none")).toBeNull();
  });
  it("room preset", () => {
    expect(cameraForIso("room")).toEqual({ position: [6, 5, 6], fov: 35 });
  });
  it("city preset", () => {
    expect(cameraForIso("city")).toEqual({ position: [12, 10, 12], fov: 30 });
  });
  it("product preset", () => {
    expect(cameraForIso("product")).toEqual({ position: [3, 2.5, 3], fov: 40 });
  });
});
