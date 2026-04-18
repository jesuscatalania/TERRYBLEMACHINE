import * as fabric from "fabric";
import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import type { TextStyle } from "@/components/typography/TextLogoControls";
import { type GoogleFont, injectGoogleFont } from "@/lib/googleFonts";

/**
 * Convert a kerning value in px (as used by TextLogoControls) to Fabric's
 * charSpacing unit (1/1000 of an em). Fabric docs:
 * https://fabricjs.com/docs/fabric.Textbox.html#charSpacing
 */
function charSpacingFromPx(px: number, fontSize: number): number {
  if (fontSize <= 0) return 0;
  return Math.round((px * 1000) / fontSize);
}

/**
 * Imperative handle for the SVG editor canvas. Mirrors the drei-pattern used
 * by `FabricCanvas` (graphic2d) so the parent page can drive the canvas
 * without re-renders. Intentionally lighter than `FabricCanvas` — the
 * typography module only needs load/serialize for now; full text-to-SVG
 * editing is deferred polish.
 *
 * Note on `TextStyle.tracking`: the `tracking` (word-spacing, px) field is
 * intentionally NOT applied to the Fabric Textbox. Fabric v6 does not expose
 * a word-spacing prop, and the usual workaround — string-mangling the input
 * via `text.split(" ").join(" ".repeat(n))` — mutates the user's content
 * and breaks copy/paste. The prop remains on the `TextStyle` shape as a
 * reservation for future work (e.g. rendering into per-word sub-Textboxes
 * or switching to a DOM-based editor).
 */
export interface SvgEditorHandle {
  canvas: () => fabric.Canvas | null;
  loadSvg: (svg: string, width: number, height: number) => Promise<void>;
  toSvgString: () => string;
  /**
   * Append a new text logo to the canvas, styled per the caller's TextStyle.
   * Uses the canvas's current content (if any) as a visual anchor — text
   * lands roughly centered. Returns the new Textbox so the caller can track
   * it if needed, or null if the canvas isn't mounted.
   */
  addText: (text: string, style: TextStyle) => Promise<fabric.Textbox | null>;
  /**
   * Apply a TextStyle patch to the currently-active Fabric object if it's
   * a Textbox. No-op if the active object isn't a Textbox (or nothing is
   * selected). Returns true if an update was applied.
   */
  updateText: (style: TextStyle) => Promise<boolean>;
}

export interface SvgEditorProps {
  /** Initial canvas width in px. Defaults to 600. */
  width?: number;
  /** Initial canvas height in px. Defaults to 400. */
  height?: number;
  className?: string;
}

export const SvgEditor = forwardRef<SvgEditorHandle, SvgEditorProps>(function SvgEditorImpl(
  { width = 600, height = 400, className },
  ref,
) {
  const canvasElRef = useRef<HTMLCanvasElement | null>(null);
  const canvasRef = useRef<fabric.Canvas | null>(null);
  // Capture the initial prop values so the once-only canvas-init effect
  // doesn't need them in its dep array — matches the pattern in
  // `graphic2d/FabricCanvas.tsx` and avoids a biome-ignore comment.
  const initialSizeRef = useRef({ width, height });
  // Track the last Textbox we created so `updateText` can patch it even
  // after `loadSvg` has stolen the active-object crown (the SVG group
  // becomes the selection after vectorize). Without this fallback, sliding
  // the kerning/size sliders after a vectorize silently no-ops until the
  // user re-clicks the textbox — a surprising UX cliff.
  const lastTextRef = useRef<fabric.Textbox | null>(null);

  useEffect(() => {
    if (!canvasElRef.current) return;
    const { width: w, height: h } = initialSizeRef.current;
    const c = new fabric.Canvas(canvasElRef.current, {
      width: w,
      height: h,
      backgroundColor: "#F7F7F8",
      preserveObjectStacking: true,
    });
    canvasRef.current = c;
    return () => {
      c.dispose();
      canvasRef.current = null;
    };
  }, []);

  useImperativeHandle(
    ref,
    (): SvgEditorHandle => ({
      canvas: () => canvasRef.current,
      async loadSvg(svg, width, height) {
        const c = canvasRef.current;
        if (!c) return;
        c.clear();
        // `c.clear()` removed any previous Textbox — forget the ref so
        // `updateText`'s fallback doesn't patch a detached object.
        lastTextRef.current = null;
        c.setDimensions({ width, height });
        c.backgroundColor = "#F7F7F8";
        const result = await fabric.loadSVGFromString(svg);
        // `result.objects` may contain nulls for unsupported SVG nodes —
        // filter them out before grouping so fabric.Group doesn't choke.
        const objects = (result.objects ?? []).filter((o): o is fabric.Object => o != null);
        if (objects.length > 0) {
          const group = new fabric.Group(objects, {
            left: 0,
            top: 0,
          });
          // Defensive scale: if the vectorizer's reported width diverges
          // from the SVG's intrinsic viewBox (e.g. a bounding-box heuristic
          // vs. the real artwork box), `scaleToWidth` re-fits the group to
          // the canvas so the user sees the artwork at the expected size.
          // Fabric preserves aspect ratio implicitly.
          group.scaleToWidth(width);
          c.add(group);
          c.setActiveObject(group);
        }
        c.requestRenderAll();
      },
      toSvgString() {
        const c = canvasRef.current;
        return c?.toSVG() ?? "";
      },
      async addText(text, style) {
        const c = canvasRef.current;
        if (!c) return null;
        // Ensure the chosen font is actually loaded before the Textbox
        // measures its metrics — otherwise first render uses the system
        // fallback and we'd need a reflow. Mirrors TextLogoControls's
        // pattern.
        await injectGoogleFont(style.font as GoogleFont);
        const tb = new fabric.Textbox(text, {
          originX: "center",
          originY: "center",
          fontFamily: style.font,
          fontSize: style.size,
          fill: style.color,
          // Fabric's Textbox uses `charSpacing` (1/1000 em units) for
          // kerning. Our kerning prop is in px; convert via the helper.
          charSpacing: charSpacingFromPx(style.kerning, style.size),
          // `style.tracking` (word-spacing) is intentionally not applied —
          // see the SvgEditorHandle TSDoc for why.
        });
        c.add(tb);
        // Use viewportCenterObject so the Textbox lands at the canvas's
        // CURRENT center rather than a stale `c.getWidth()/2` snapshot.
        // Matters when the user clicks "Add text" before "Vectorize" —
        // `loadSvg` will later resize the canvas, but viewportCenterObject
        // reflects the dimensions at insertion time.
        c.viewportCenterObject(tb);
        c.setActiveObject(tb);
        c.requestRenderAll();
        lastTextRef.current = tb;
        return tb;
      },
      async updateText(style) {
        const c = canvasRef.current;
        if (!c) return false;
        // Prefer the currently-active Textbox, but fall back to the last
        // one we created so post-vectorize slider changes still apply.
        const active = c.getActiveObject();
        const target = active instanceof fabric.Textbox ? active : lastTextRef.current;
        if (!target) return false;
        // If the fallback textbox was removed from the canvas (e.g. via
        // `c.clear()` in loadSvg), skip silently — patching a detached
        // object would mislead the caller.
        if (!c.getObjects().includes(target)) {
          lastTextRef.current = null;
          return false;
        }
        await injectGoogleFont(style.font as GoogleFont);
        target.set({
          fontFamily: style.font,
          fontSize: style.size,
          fill: style.color,
          charSpacing: charSpacingFromPx(style.kerning, style.size),
        });
        c.requestRenderAll();
        return true;
      },
    }),
    [],
  );

  return (
    <div
      className={`flex h-full w-full items-center justify-center bg-neutral-dark-950 ${className ?? ""}`}
    >
      <canvas
        ref={canvasElRef}
        data-testid="svg-editor-canvas"
        className="rounded-xs border border-neutral-dark-700"
      />
    </div>
  );
});
