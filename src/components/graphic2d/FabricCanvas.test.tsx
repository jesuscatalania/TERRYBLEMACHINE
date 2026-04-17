import { render } from "@testing-library/react";
import { createRef } from "react";
import { describe, expect, it } from "vitest";
import { FabricCanvas, type FabricCanvasHandle } from "@/components/graphic2d/FabricCanvas";

// vitest-canvas-mock (wired in src/test/setup.ts) patches
// HTMLCanvasElement.getContext() so fabric.Canvas can be instantiated under
// jsdom. Un-skipped as part of FU #104.
describe("FabricCanvas handle", () => {
  it("exposes flipH/flipV/setCanvasSize/cropToSelection/enter*Select/exitSelectionMode", () => {
    const ref = createRef<FabricCanvasHandle>();
    render(<FabricCanvas ref={ref} width={200} height={100} />);
    const h = ref.current;
    expect(h).not.toBeNull();
    if (!h) return;
    expect(typeof h.flipH).toBe("function");
    expect(typeof h.flipV).toBe("function");
    expect(typeof h.setCanvasSize).toBe("function");
    expect(typeof h.cropToSelection).toBe("function");
    expect(typeof h.enterMarqueeSelect).toBe("function");
    expect(typeof h.enterLassoSelect).toBe("function");
    expect(typeof h.exitSelectionMode).toBe("function");
    expect(typeof h.hasCropSelection).toBe("function");
  });

  it("setCanvasSize updates canvas dimensions", () => {
    const ref = createRef<FabricCanvasHandle>();
    render(<FabricCanvas ref={ref} width={200} height={100} />);
    const h = ref.current;
    expect(h).not.toBeNull();
    if (!h) return;
    h.setCanvasSize(400, 300);
    const c = h.canvas();
    expect(c?.getWidth()).toBe(400);
    expect(c?.getHeight()).toBe(300);
  });

  it("hasCropSelection is false on a fresh canvas", () => {
    const ref = createRef<FabricCanvasHandle>();
    render(<FabricCanvas ref={ref} width={200} height={100} />);
    const h = ref.current;
    expect(h).not.toBeNull();
    if (!h) return;
    expect(h.hasCropSelection()).toBe(false);
  });

  it("enterMarqueeSelect / exitSelectionMode toggle canvas interaction flags", () => {
    const ref = createRef<FabricCanvasHandle>();
    render(<FabricCanvas ref={ref} width={200} height={100} />);
    const h = ref.current;
    expect(h).not.toBeNull();
    if (!h) return;
    h.enterMarqueeSelect();
    const c = h.canvas();
    expect(c?.selection).toBe(false);
    expect(c?.isDrawingMode).toBe(false);
    h.exitSelectionMode();
    expect(c?.selection).toBe(true);
    expect(c?.isDrawingMode).toBe(false);
  });
});
