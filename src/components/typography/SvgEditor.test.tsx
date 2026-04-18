import { render } from "@testing-library/react";
import type * as fabric from "fabric";
import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { SvgEditor, type SvgEditorHandle } from "@/components/typography/SvgEditor";
import { injectGoogleFont } from "@/lib/googleFonts";

// `injectGoogleFont` awaits `document.fonts.load()` which isn't wired in
// jsdom. The function already no-ops when `document.fonts` is undefined,
// but mocking it explicitly keeps the text-method tests deterministic and
// avoids flakiness from the (patched) jsdom fonts API.
vi.mock("@/lib/googleFonts", async () => {
  const actual = await vi.importActual<typeof import("@/lib/googleFonts")>("@/lib/googleFonts");
  return {
    ...actual,
    injectGoogleFont: vi.fn().mockResolvedValue(undefined),
  };
});

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

describe("SvgEditor text methods", () => {
  const BASE_STYLE = {
    font: "Inter" as const,
    color: "#F7F7F8",
    size: 72,
    kerning: 0,
    tracking: 0,
  };

  it("addText appends a Textbox with the given style and selects it", async () => {
    const ref = createRef<SvgEditorHandle>();
    render(<SvgEditor ref={ref} />);
    const tb = await ref.current?.addText("Acme", BASE_STYLE);
    expect(tb).not.toBeNull();
    expect(tb?.fontFamily).toBe("Inter");
    expect(tb?.fontSize).toBe(72);
    expect(tb?.fill).toBe("#F7F7F8");
    const c = ref.current?.canvas();
    expect(c?.getActiveObject()).toBe(tb);
  });

  it("updateText returns false when active object is not a Textbox", async () => {
    const ref = createRef<SvgEditorHandle>();
    render(<SvgEditor ref={ref} />);
    const ok = await ref.current?.updateText(BASE_STYLE);
    expect(ok).toBe(false);
  });

  it("updateText applies style patch when a Textbox is selected", async () => {
    const ref = createRef<SvgEditorHandle>();
    render(<SvgEditor ref={ref} />);
    await ref.current?.addText("Acme", BASE_STYLE);
    const ok = await ref.current?.updateText({
      ...BASE_STYLE,
      size: 120,
      color: "#e85d2d",
    });
    expect(ok).toBe(true);
    const active = ref.current?.canvas()?.getActiveObject() as fabric.Textbox;
    expect(active.fontSize).toBe(120);
    expect(active.fill).toBe("#e85d2d");
  });

  it("kerning in px converts to Fabric charSpacing (1/1000 em)", async () => {
    const ref = createRef<SvgEditorHandle>();
    render(<SvgEditor ref={ref} />);
    // kerning=10px at fontSize=100 → 100 em units (10 * 1000 / 100)
    const tb = await ref.current?.addText("A", { ...BASE_STYLE, size: 100, kerning: 10 });
    expect(tb?.charSpacing).toBe(100);
  });

  it("updateText falls back to last-created Textbox when active object is not text", async () => {
    const ref = createRef<SvgEditorHandle>();
    render(<SvgEditor ref={ref} />);
    const tb = await ref.current?.addText("Acme", BASE_STYLE);
    // Simulate a post-vectorize state where the group (or anything non-text)
    // becomes the active object — updateText must still patch the Textbox.
    const c = ref.current?.canvas();
    c?.discardActiveObject();
    const ok = await ref.current?.updateText({ ...BASE_STYLE, size: 144 });
    expect(ok).toBe(true);
    expect(tb?.fontSize).toBe(144);
  });

  it("updateText skips redundant font injection when font hasn't changed", async () => {
    const mock = vi.mocked(injectGoogleFont);
    mock.mockClear();
    const ref = createRef<SvgEditorHandle>();
    render(<SvgEditor ref={ref} />);
    await ref.current?.addText("Acme", BASE_STYLE); // call 1 — injects Inter
    expect(mock).toHaveBeenCalledTimes(1);
    await ref.current?.updateText({ ...BASE_STYLE, size: 120 }); // same font
    expect(mock).toHaveBeenCalledTimes(1); // not called again
    await ref.current?.updateText({ ...BASE_STYLE, font: "Roboto" }); // different font
    expect(mock).toHaveBeenCalledTimes(2);
  });
});
