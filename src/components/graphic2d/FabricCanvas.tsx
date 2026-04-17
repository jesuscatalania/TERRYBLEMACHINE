import * as fabric from "fabric";
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
          (img as unknown as { data: { id: string } }).data = { id };
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
