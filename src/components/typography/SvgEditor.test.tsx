import { render } from "@testing-library/react";
import { createRef } from "react";
import { describe, expect, it } from "vitest";
import { SvgEditor, type SvgEditorHandle } from "@/components/typography/SvgEditor";

// vitest-canvas-mock (wired in src/test/setup.ts) patches
// HTMLCanvasElement.getContext() so fabric.Canvas can be instantiated under
// jsdom — same pattern as FabricCanvas.test.tsx.
describe("SvgEditor", () => {
  it("exposes handle methods", () => {
    const ref = createRef<SvgEditorHandle>();
    render(<SvgEditor ref={ref} />);
    expect(typeof ref.current?.loadSvg).toBe("function");
    expect(typeof ref.current?.toSvgString).toBe("function");
    expect(ref.current?.canvas()).not.toBeNull();
  });

  it("loadSvg + toSvgString roundtrip", async () => {
    const ref = createRef<SvgEditorHandle>();
    render(<SvgEditor ref={ref} />);
    const handle = ref.current;
    expect(handle).not.toBeNull();
    if (!handle) return;
    const svg =
      '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect width="100" height="100" fill="#e85d2d"/></svg>';
    await handle.loadSvg(svg, 100, 100);
    const out = handle.toSvgString();
    expect(out).toContain("<svg");
    expect(out.length).toBeGreaterThan(50);
  });

  it("loadSvg resizes the canvas to the given dimensions", async () => {
    const ref = createRef<SvgEditorHandle>();
    render(<SvgEditor ref={ref} />);
    const handle = ref.current;
    expect(handle).not.toBeNull();
    if (!handle) return;
    const svg =
      '<svg xmlns="http://www.w3.org/2000/svg" width="250" height="180"><rect width="250" height="180" fill="#222"/></svg>';
    await handle.loadSvg(svg, 250, 180);
    const c = handle.canvas();
    expect(c?.getWidth()).toBe(250);
    expect(c?.getHeight()).toBe(180);
  });
});
