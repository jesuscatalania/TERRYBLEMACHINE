import * as fabric from "fabric";
import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";

/**
 * Imperative handle for the SVG editor canvas. Mirrors the drei-pattern used
 * by `FabricCanvas` (graphic2d) so the parent page can drive the canvas
 * without re-renders. Intentionally lighter than `FabricCanvas` — the
 * typography module only needs load/serialize for now; full text-to-SVG
 * editing is deferred polish.
 */
export interface SvgEditorHandle {
  canvas: () => fabric.Canvas | null;
  loadSvg: (svg: string, width: number, height: number) => Promise<void>;
  toSvgString: () => string;
}

export interface SvgEditorProps {
  className?: string;
}

export const SvgEditor = forwardRef<SvgEditorHandle, SvgEditorProps>(function SvgEditorImpl(
  { className },
  ref,
) {
  const canvasElRef = useRef<HTMLCanvasElement | null>(null);
  const canvasRef = useRef<fabric.Canvas | null>(null);

  useEffect(() => {
    if (!canvasElRef.current) return;
    const c = new fabric.Canvas(canvasElRef.current, {
      width: 600,
      height: 400,
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
          c.add(group);
          c.setActiveObject(group);
        }
        c.requestRenderAll();
      },
      toSvgString() {
        const c = canvasRef.current;
        return c?.toSVG() ?? "";
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
