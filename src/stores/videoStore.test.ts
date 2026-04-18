import { beforeEach, describe, expect, it } from "vitest";
import { useVideoStore } from "@/stores/videoStore";

describe("videoStore", () => {
  beforeEach(() => useVideoStore.getState().reset());

  it("adds segments with unique ids", () => {
    const a = useVideoStore.getState().addSegment({ kind: "ai", label: "shot 1", duration_s: 5 });
    const b = useVideoStore.getState().addSegment({ kind: "ai", label: "shot 2", duration_s: 5 });
    expect(a).not.toBe(b);
    expect(useVideoStore.getState().segments).toHaveLength(2);
  });

  it("removes segments", () => {
    const id = useVideoStore.getState().addSegment({ kind: "ai", label: "x", duration_s: 3 });
    useVideoStore.getState().removeSegment(id);
    expect(useVideoStore.getState().segments).toHaveLength(0);
  });

  it("updates segments", () => {
    const id = useVideoStore.getState().addSegment({ kind: "ai", label: "x", duration_s: 3 });
    useVideoStore.getState().updateSegment(id, { label: "y", busy: true });
    const seg = useVideoStore.getState().segments[0];
    expect(seg?.label).toBe("y");
    expect(seg?.busy).toBe(true);
  });

  it("moves segments", () => {
    const a = useVideoStore.getState().addSegment({ kind: "ai", label: "a", duration_s: 3 });
    const b = useVideoStore.getState().addSegment({ kind: "ai", label: "b", duration_s: 3 });
    useVideoStore.getState().moveSegment(0, 1);
    const ids = useVideoStore.getState().segments.map((s) => s.id);
    expect(ids).toEqual([b, a]);
  });

  it("moveSegment ignores out-of-range indices", () => {
    useVideoStore.getState().addSegment({ kind: "ai", label: "a", duration_s: 3 });
    const before = useVideoStore.getState().segments;
    useVideoStore.getState().moveSegment(5, 0);
    expect(useVideoStore.getState().segments).toEqual(before);
  });

  it("applyVideoResult updates segment + clears busy/error", () => {
    const id = useVideoStore.getState().addSegment({
      kind: "ai",
      label: "x",
      duration_s: 5,
      busy: true,
      error: "nope",
    });
    useVideoStore.getState().applyVideoResult(id, {
      video_url: "u",
      local_path: "/p",
      model: "Kling20",
      duration_s: 5,
    });
    const seg = useVideoStore.getState().segments[0];
    expect(seg?.busy).toBe(false);
    expect(seg?.error).toBeUndefined();
    expect(seg?.video_url).toBe("u");
    expect(seg?.local_path).toBe("/p");
  });
});
