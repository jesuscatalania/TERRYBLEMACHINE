import { render } from "@testing-library/react";
import { createRef } from "react";
import { describe, expect, it } from "vitest";
import { FabricCanvas, type FabricCanvasHandle } from "@/components/graphic2d/FabricCanvas";

// jsdom does not implement HTMLCanvasElement.getContext(). fabric.Canvas
// calls it during construction, so every test in this file would throw at
// render time. We keep the assertions so they serve as documentation of the
// handle surface, but skip them until we have a proper canvas shim.
//
// TODO(#104, #106): once vitest-canvas-mock is wired in, un-skip this suite
// and add a toGif() empty-canvas test asserting the promise resolves rather
// than hangs (covers the abort / FileReader.onerror / synchronous-throw
// paths added in the T19 hardening pass).
describe.skip("FabricCanvas handle", () => {
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
