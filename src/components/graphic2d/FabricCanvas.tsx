import * as fabric from "fabric";
import GIF from "gif.js";
import { jsPDF } from "jspdf";
import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";

export interface FabricLayer {
  id: string;
  label: string;
  type: "image" | "text" | "shape";
  visible: boolean;
  locked: boolean;
}

/**
 * Imperative handle exposed to the parent so the toolbar + layer panel can
 * drive the canvas without re-renders on every pointer move.
 */
export interface FabricCanvasHandle {
  canvas: () => fabric.Canvas | null;
  addImageFromUrl: (url: string) => Promise<FabricLayer>;
  addText: (text: string) => FabricLayer;
  layers: () => FabricLayer[];
  removeLayer: (id: string) => void;
  toggleVisibility: (id: string) => void;
  toggleLock: (id: string) => void;
  applyFilter: (
    id: string,
    filter: "blur" | "sharpen" | "brightness" | "contrast" | "saturation",
    intensity: number,
  ) => void;
  toPng: (withTransparency: boolean) => string;
  toJpeg: (quality: number) => string;
  toWebp: (quality: number) => string;
  toSvg: () => string;
  /** Export current canvas as a single-page PDF data URL. */
  toPdf: () => string;
  /** Export canvas as GIF data URL. Static when frames<=1, animated otherwise. */
  toGif: (options?: { frames?: number; delayMs?: number }) => Promise<string>;
  /** Enter free-drawing mode — pointer strokes become mask paths. */
  enterMaskMode: () => void;
  /** Exit free-drawing mode. Does not remove existing mask strokes. */
  exitMaskMode: () => void;
  /** True if at least one mask stroke exists on the canvas. */
  hasMask: () => boolean;
  /** Remove every mask stroke from the canvas. */
  clearMask: () => void;
  /**
   * Export only the mask layer as a PNG data-URL: white strokes on
   * pure black, same dimensions as the canvas. Non-mask objects are
   * hidden during the export and restored afterwards.
   */
  getMaskDataUrl: () => string;
  /**
   * URL of the first image layer on the canvas (usually the source image
   * for inpaint) or null if none exists. Used by the Inpaint flow to
   * detect data-URL sources that fal.ai cannot ingest.
   */
  getFirstImageUrl: () => string | null;
  /** Flip the given object horizontally. */
  flipH: (id: string) => void;
  /** Flip the given object vertically. */
  flipV: (id: string) => void;
  /** Resize the canvas. Does not rescale existing objects. */
  setCanvasSize: (width: number, height: number) => void;
  /** Crop the canvas to the current crop selection's bounding box. */
  cropToSelection: () => void;
  /** Enter marquee-select mode — drag to create a crop bounding rectangle. */
  enterMarqueeSelect: () => void;
  /** Enter lasso-select mode — free-hand draw to create a crop polygon (uses bounding box). */
  enterLassoSelect: () => void;
  /** Exit any selection mode. */
  exitSelectionMode: () => void;
  /** True if at least one crop selection exists on the canvas. */
  hasCropSelection: () => boolean;
  /** Patch font / color / size on a Textbox layer. Missing keys are left unchanged. */
  updateText: (id: string, patch: { font?: string; color?: string; size?: number }) => void;
}

export interface FabricCanvasProps {
  width?: number;
  height?: number;
  /** Fires whenever the layer list changes. */
  onLayersChange?: (layers: FabricLayer[]) => void;
  /** Fires whenever selection changes. */
  onSelectionChange?: (id: string | null) => void;
  className?: string;
}

export const FabricCanvas = forwardRef<FabricCanvasHandle, FabricCanvasProps>(
  function FabricCanvasImpl(props, ref) {
    const { width = 1024, height = 768, onLayersChange, onSelectionChange, className = "" } = props;
    const canvasElRef = useRef<HTMLCanvasElement | null>(null);
    const canvasRef = useRef<fabric.Canvas | null>(null);
    const layersRef = useRef<FabricLayer[]>([]);
    const maskModeRef = useRef(false);
    const selectionModeRef = useRef<"marquee" | "lasso" | null>(null);
    const marqueeStateRef = useRef<{
      start: { x: number; y: number } | null;
      rect: fabric.Rect | null;
    }>({ start: null, rect: null });
    const onLayersChangeRef = useRef(onLayersChange);
    const onSelectionChangeRef = useRef(onSelectionChange);
    const initialSizeRef = useRef({ width, height });
    onLayersChangeRef.current = onLayersChange;
    onSelectionChangeRef.current = onSelectionChange;

    useEffect(() => {
      if (!canvasElRef.current) return;
      const { width: w, height: h } = initialSizeRef.current;
      const c = new fabric.Canvas(canvasElRef.current, {
        width: w,
        height: h,
        backgroundColor: "#0E0E11",
        preserveObjectStacking: true,
      });
      canvasRef.current = c;

      c.on("selection:created", (e) => onSelectionChangeRef.current?.(objId(e.selected?.[0])));
      c.on("selection:updated", (e) => onSelectionChangeRef.current?.(objId(e.selected?.[0])));
      c.on("selection:cleared", () => onSelectionChangeRef.current?.(null));

      // Tag paths drawn while in mask mode so we can isolate / clear them.
      // When in lasso-select mode, replace the freeform path with a bounding-box
      // fabric.Rect tagged __cropSelection so cropToSelection can find it.
      c.on("path:created", (e: unknown) => {
        const path = (e as { path?: fabric.Object }).path;
        if (!path) return;
        if (maskModeRef.current) {
          (path as unknown as { __mask: boolean }).__mask = true;
          path.selectable = false;
          path.evented = false;
          return;
        }
        if (selectionModeRef.current === "lasso") {
          const bounds = path.getBoundingRect();
          c.remove(path);
          const rect = new fabric.Rect({
            left: bounds.left,
            top: bounds.top,
            width: bounds.width,
            height: bounds.height,
            fill: "rgba(232,93,45,0.1)",
            stroke: "rgba(232,93,45,0.6)",
            strokeWidth: 1,
            selectable: false,
            evented: false,
          });
          (rect as unknown as { __cropSelection: boolean }).__cropSelection = true;
          c.add(rect);
          c.requestRenderAll();
        }
      });

      // Marquee-select listeners: installed once, gated on selectionModeRef.
      c.on("mouse:down", (opt: unknown) => {
        if (selectionModeRef.current !== "marquee") return;
        const e = (opt as { e: MouseEvent }).e;
        const p = c.getScenePoint(e);
        // Remove any previous crop selection so only one exists at a time.
        clearCropSelections(c);
        const rect = new fabric.Rect({
          left: p.x,
          top: p.y,
          width: 0,
          height: 0,
          fill: "rgba(232,93,45,0.1)",
          stroke: "rgba(232,93,45,0.6)",
          strokeWidth: 1,
          selectable: false,
          evented: false,
        });
        (rect as unknown as { __cropSelection: boolean }).__cropSelection = true;
        marqueeStateRef.current.start = p;
        marqueeStateRef.current.rect = rect;
        c.add(rect);
      });
      c.on("mouse:move", (opt: unknown) => {
        if (selectionModeRef.current !== "marquee") return;
        const { start, rect } = marqueeStateRef.current;
        if (!start || !rect) return;
        const e = (opt as { e: MouseEvent }).e;
        const p = c.getScenePoint(e);
        rect.set({
          left: Math.min(start.x, p.x),
          top: Math.min(start.y, p.y),
          width: Math.abs(p.x - start.x),
          height: Math.abs(p.y - start.y),
        });
        c.requestRenderAll();
      });
      c.on("mouse:up", () => {
        if (selectionModeRef.current !== "marquee") return;
        marqueeStateRef.current.start = null;
        marqueeStateRef.current.rect = null;
      });

      return () => {
        c.dispose();
        canvasRef.current = null;
      };
    }, []);

    useEffect(() => {
      canvasRef.current?.setDimensions({ width, height });
    }, [width, height]);

    const refreshLayers = () => {
      if (!canvasRef.current) return;
      const objs = canvasRef.current.getObjects();
      layersRef.current = objs.map((o, i) => ({
        id: getOrAssignId(o, i),
        label: labelFor(o, i),
        type: typeFor(o),
        visible: o.visible !== false,
        locked: Boolean(o.lockMovementX && o.lockMovementY),
      }));
      onLayersChangeRef.current?.(layersRef.current);
    };

    // biome-ignore lint/correctness/useExhaustiveDependencies: refreshLayers is stable via refs
    useImperativeHandle(
      ref,
      (): FabricCanvasHandle => ({
        canvas: () => canvasRef.current,
        async addImageFromUrl(url) {
          const c = canvasRef.current;
          if (!c) throw new Error("canvas not ready");
          const img = await fabric.FabricImage.fromURL(url, {
            crossOrigin: "anonymous",
          });
          const scale = Math.min(
            c.getWidth() / (img.width ?? 1),
            c.getHeight() / (img.height ?? 1),
            1,
          );
          img.scale(scale);
          const id = newId("img");
          (img as unknown as { data: { id: string; sourceUrl: string } }).data = {
            id,
            sourceUrl: url,
          };
          c.add(img);
          c.setActiveObject(img);
          c.requestRenderAll();
          refreshLayers();
          return {
            id,
            label: `Image ${layersRef.current.length}`,
            type: "image",
            visible: true,
            locked: false,
          };
        },
        addText(text) {
          const c = canvasRef.current;
          if (!c) throw new Error("canvas not ready");
          const obj = new fabric.Textbox(text, {
            left: 100,
            top: 100,
            fontSize: 48,
            fill: "#F7F7F8",
            fontFamily: "Inter",
            width: 480,
          });
          const id = newId("text");
          (obj as unknown as { data: { id: string } }).data = { id };
          c.add(obj);
          c.setActiveObject(obj);
          c.requestRenderAll();
          refreshLayers();
          return {
            id,
            label: text.slice(0, 24),
            type: "text",
            visible: true,
            locked: false,
          };
        },
        layers: () => layersRef.current,
        removeLayer(id) {
          const c = canvasRef.current;
          if (!c) return;
          const obj = findById(c, id);
          if (obj) {
            c.remove(obj);
            c.requestRenderAll();
            refreshLayers();
          }
        },
        toggleVisibility(id) {
          const c = canvasRef.current;
          if (!c) return;
          const obj = findById(c, id);
          if (obj) {
            obj.visible = !obj.visible;
            c.requestRenderAll();
            refreshLayers();
          }
        },
        toggleLock(id) {
          const c = canvasRef.current;
          if (!c) return;
          const obj = findById(c, id);
          if (obj) {
            const locked = !obj.lockMovementX;
            obj.lockMovementX = locked;
            obj.lockMovementY = locked;
            obj.lockScalingX = locked;
            obj.lockScalingY = locked;
            obj.lockRotation = locked;
            obj.selectable = !locked;
            c.requestRenderAll();
            refreshLayers();
          }
        },
        applyFilter(id, filter, intensity) {
          const c = canvasRef.current;
          if (!c) return;
          const obj = findById(c, id);
          if (!obj || !(obj instanceof fabric.FabricImage)) return;
          const filters = filtersFor(filter, intensity);
          obj.filters = filters;
          obj.applyFilters();
          c.requestRenderAll();
        },
        toPng(withTransparency) {
          const c = canvasRef.current;
          if (!c) return "";
          const prevBg = c.backgroundColor;
          if (withTransparency) c.backgroundColor = "";
          const url = c.toDataURL({ format: "png", multiplier: 1 });
          c.backgroundColor = prevBg;
          return url;
        },
        toJpeg(quality) {
          const c = canvasRef.current;
          if (!c) return "";
          return c.toDataURL({
            format: "jpeg",
            quality: Math.min(1, Math.max(0.1, quality)),
            multiplier: 1,
          });
        },
        toWebp(quality) {
          const c = canvasRef.current;
          if (!c) return "";
          // Fabric uses the same API; not every browser supports webp but
          // Tauri's WKWebView does.
          return c.toDataURL({
            format: "webp" as "png",
            quality: Math.min(1, Math.max(0.1, quality)),
            multiplier: 1,
          });
        },
        toSvg() {
          return canvasRef.current?.toSVG() ?? "";
        },
        toPdf() {
          const c = canvasRef.current;
          if (!c) return "";
          const pngUrl = c.toDataURL({ format: "png", multiplier: 1 });
          const w = c.getWidth();
          const h = c.getHeight();
          const pdf = new jsPDF({
            orientation: w >= h ? "landscape" : "portrait",
            unit: "px",
            format: [w, h],
          });
          pdf.addImage(pngUrl, "PNG", 0, 0, w, h);
          return pdf.output("dataurlstring");
        },
        toGif({ frames = 1, delayMs = 100 } = {}) {
          return new Promise<string>((resolve) => {
            const c = canvasRef.current;
            if (!c) {
              resolve("");
              return;
            }
            const gif = new GIF({
              workers: 2,
              quality: 10,
              width: c.getWidth(),
              height: c.getHeight(),
              workerScript: "/gif.worker.js",
            });
            const count = Math.max(1, frames);
            for (let i = 0; i < count; i++) {
              gif.addFrame(c.lowerCanvasEl, { copy: true, delay: delayMs });
            }
            gif.on("finished", (blob: Blob) => {
              const r = new FileReader();
              r.onload = () => resolve(r.result as string);
              r.readAsDataURL(blob);
            });
            gif.render();
          });
        },
        enterMaskMode() {
          const c = canvasRef.current;
          if (!c) return;
          const brush = new fabric.PencilBrush(c);
          brush.color = "rgba(255,255,255,0.85)";
          brush.width = 40;
          c.freeDrawingBrush = brush;
          c.isDrawingMode = true;
          c.discardActiveObject();
          maskModeRef.current = true;
        },
        exitMaskMode() {
          const c = canvasRef.current;
          if (!c) return;
          c.isDrawingMode = false;
          maskModeRef.current = false;
        },
        hasMask() {
          const c = canvasRef.current;
          if (!c) return false;
          return c.getObjects().some((o) => (o as unknown as { __mask?: boolean }).__mask === true);
        },
        clearMask() {
          const c = canvasRef.current;
          if (!c) return;
          for (const o of c.getObjects()) {
            if ((o as unknown as { __mask?: boolean }).__mask === true) {
              c.remove(o);
            }
          }
          c.requestRenderAll();
        },
        getMaskDataUrl() {
          const c = canvasRef.current;
          if (!c) return "";
          // Hide every non-mask object, paint the background pure black, export,
          // then restore the previous visibility + background.
          const originals: Array<{ obj: fabric.Object; vis: boolean }> = [];
          for (const obj of c.getObjects()) {
            const isMask = (obj as unknown as { __mask?: boolean }).__mask === true;
            originals.push({ obj, vis: obj.visible ?? true });
            obj.visible = isMask;
          }
          const prevBg = c.backgroundColor;
          c.backgroundColor = "#000000";
          c.requestRenderAll();
          const url = c.toDataURL({ format: "png", multiplier: 1 });
          for (const { obj, vis } of originals) {
            obj.visible = vis;
          }
          c.backgroundColor = prevBg;
          c.requestRenderAll();
          return url;
        },
        getFirstImageUrl() {
          const c = canvasRef.current;
          if (!c) return null;
          for (const o of c.getObjects()) {
            if (o instanceof fabric.FabricImage) {
              const data = (o as unknown as { data?: { sourceUrl?: string } }).data;
              if (data?.sourceUrl) return data.sourceUrl;
            }
          }
          return null;
        },
        flipH(id) {
          const c = canvasRef.current;
          if (!c) return;
          const obj = findById(c, id);
          if (obj) {
            obj.set({ flipX: !obj.flipX });
            c.requestRenderAll();
          }
        },
        flipV(id) {
          const c = canvasRef.current;
          if (!c) return;
          const obj = findById(c, id);
          if (obj) {
            obj.set({ flipY: !obj.flipY });
            c.requestRenderAll();
          }
        },
        setCanvasSize(w, h) {
          const c = canvasRef.current;
          if (!c) return;
          c.setDimensions({ width: w, height: h });
          c.requestRenderAll();
        },
        cropToSelection() {
          const c = canvasRef.current;
          if (!c) return;
          const sel = c
            .getObjects()
            .find((o) => (o as unknown as { __cropSelection?: boolean }).__cropSelection) as
            | fabric.Rect
            | undefined;
          if (!sel) return;
          const left = sel.left ?? 0;
          const top = sel.top ?? 0;
          const width = (sel.width ?? 0) * (sel.scaleX ?? 1);
          const height = (sel.height ?? 0) * (sel.scaleY ?? 1);
          if (width <= 0 || height <= 0) return;
          // Translate every non-selection object to the new origin, then
          // remove the crop marker and resize the canvas.
          const toRemove: fabric.Object[] = [];
          for (const o of c.getObjects()) {
            if ((o as unknown as { __cropSelection?: boolean }).__cropSelection) {
              toRemove.push(o);
              continue;
            }
            o.set({
              left: (o.left ?? 0) - left,
              top: (o.top ?? 0) - top,
            });
            o.setCoords();
          }
          for (const o of toRemove) c.remove(o);
          c.setDimensions({ width, height });
          c.requestRenderAll();
          refreshLayers();
        },
        enterMarqueeSelect() {
          const c = canvasRef.current;
          if (!c) return;
          selectionModeRef.current = "marquee";
          c.isDrawingMode = false;
          c.selection = false;
          c.defaultCursor = "crosshair";
          c.discardActiveObject();
          c.requestRenderAll();
        },
        enterLassoSelect() {
          const c = canvasRef.current;
          if (!c) return;
          selectionModeRef.current = "lasso";
          const brush = new fabric.PencilBrush(c);
          brush.color = "rgba(232,93,45,0.6)";
          brush.width = 2;
          c.freeDrawingBrush = brush;
          c.isDrawingMode = true;
          c.selection = false;
          c.defaultCursor = "crosshair";
          c.discardActiveObject();
          c.requestRenderAll();
        },
        exitSelectionMode() {
          const c = canvasRef.current;
          if (!c) return;
          selectionModeRef.current = null;
          c.isDrawingMode = false;
          c.selection = true;
          c.defaultCursor = "default";
          marqueeStateRef.current.start = null;
          marqueeStateRef.current.rect = null;
        },
        hasCropSelection() {
          const c = canvasRef.current;
          if (!c) return false;
          return c
            .getObjects()
            .some((o) => (o as unknown as { __cropSelection?: boolean }).__cropSelection === true);
        },
        updateText(id, patch) {
          const c = canvasRef.current;
          if (!c) return;
          const obj = findById(c, id);
          if (!obj || !(obj instanceof fabric.Textbox)) return;
          if (patch.font !== undefined) obj.set({ fontFamily: patch.font });
          if (patch.color !== undefined) obj.set({ fill: patch.color });
          if (patch.size !== undefined) obj.set({ fontSize: patch.size });
          c.requestRenderAll();
          refreshLayers();
        },
      }),
      [],
    );

    return (
      <div
        className={`flex h-full w-full items-center justify-center bg-neutral-dark-950 ${className}`}
      >
        <canvas
          ref={canvasElRef}
          data-testid="fabric-canvas"
          className="rounded-xs border border-neutral-dark-700"
        />
      </div>
    );
  },
);

// ─── Helpers ────────────────────────────────────────────────────────────

let idCounter = 0;
function newId(prefix: string): string {
  idCounter += 1;
  return `${prefix}-${idCounter}`;
}

function objId(o: unknown): string | null {
  const data = (o as { data?: { id?: string } } | undefined)?.data;
  return data?.id ?? null;
}

function getOrAssignId(o: unknown, index: number): string {
  const data = (o as { data?: { id?: string } }).data;
  if (data?.id) return data.id;
  const fresh = `obj-${index}-${Date.now()}`;
  (o as { data: { id: string } }).data = { id: fresh };
  return fresh;
}

function labelFor(o: fabric.Object, i: number): string {
  if (o instanceof fabric.Textbox) {
    return `Text · ${o.text?.slice(0, 16) ?? ""}`;
  }
  if (o instanceof fabric.FabricImage) {
    return `Image ${i + 1}`;
  }
  return `Object ${i + 1}`;
}

function typeFor(o: fabric.Object): FabricLayer["type"] {
  if (o instanceof fabric.Textbox) return "text";
  if (o instanceof fabric.FabricImage) return "image";
  return "shape";
}

function findById(c: fabric.Canvas, id: string): fabric.Object | null {
  return c.getObjects().find((o) => objId(o) === id) ?? null;
}

function clearCropSelections(c: fabric.Canvas): void {
  const marked = c
    .getObjects()
    .filter((o) => (o as unknown as { __cropSelection?: boolean }).__cropSelection === true);
  for (const o of marked) c.remove(o);
}

function filtersFor(
  name: "blur" | "sharpen" | "brightness" | "contrast" | "saturation",
  intensity: number,
): fabric.filters.BaseFilter<string>[] {
  const clamp = (min: number, max: number) => Math.min(max, Math.max(min, intensity));
  switch (name) {
    case "blur":
      return [new fabric.filters.Blur({ blur: clamp(0, 1) })];
    case "sharpen":
      return [
        new fabric.filters.Convolute({
          matrix: [0, -1, 0, -1, 5, -1, 0, -1, 0].map((v) => (v === 5 ? 1 + clamp(0, 4) : v)),
        }),
      ];
    case "brightness":
      return [new fabric.filters.Brightness({ brightness: clamp(-1, 1) })];
    case "contrast":
      return [new fabric.filters.Contrast({ contrast: clamp(-1, 1) })];
    case "saturation":
      return [new fabric.filters.Saturation({ saturation: clamp(-1, 1) })];
  }
}
