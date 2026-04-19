import { Brush, Download, Image as ImageIcon, Plus, Sparkles, Type } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  ExportDialog,
  type ExportFormat,
  type ExportSettings,
} from "@/components/graphic2d/ExportDialog";
import {
  FabricCanvas,
  type FabricCanvasHandle,
  type FabricLayer,
} from "@/components/graphic2d/FabricCanvas";
import { LayerList } from "@/components/graphic2d/LayerList";
import { TextControls } from "@/components/graphic2d/TextControls";
import { Button } from "@/components/ui/Button";
import { Dropdown } from "@/components/ui/Dropdown";
import { HelpIcon } from "@/components/ui/HelpIcon";
import { Input } from "@/components/ui/Input";
import { LoadingButton } from "@/components/ui/LoadingButton";
import { OptimizeToggle } from "@/components/ui/OptimizeToggle";
import { Skeleton } from "@/components/ui/Skeleton";
import { ToolDropdown } from "@/components/ui/ToolDropdown";
import { useOptimizePrompt } from "@/hooks/useOptimizePrompt";
import { formatError } from "@/lib/formatError";
import { generateVariants, type ImageResult, inpaintImage, isDataUrl } from "@/lib/imageCommands";
import { parseOverride, resolveOverrideToModel } from "@/lib/promptOverride";
import { useAppStore } from "@/stores/appStore";
import { useUiStore } from "@/stores/uiStore";

export function Graphic2DPage() {
  const canvasRef = useRef<FabricCanvasHandle>(null);
  const [prompt, setPrompt] = useState("");
  const [busy, setBusy] = useState(false);
  const [variants, setVariants] = useState<ImageResult[]>([]);
  const [layers, setLayers] = useState<FabricLayer[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [exportOpen, setExportOpen] = useState(false);
  const [filter, setFilter] = useState<
    "blur" | "sharpen" | "brightness" | "contrast" | "saturation"
  >("blur");
  const [maskMode, setMaskMode] = useState(false);
  const [inpaintPromptOpen, setInpaintPromptOpen] = useState(false);
  const [inpaintPrompt, setInpaintPrompt] = useState("");
  const [inpaintBusy, setInpaintBusy] = useState(false);
  const [selectionMode, setSelectionMode] = useState<"none" | "marquee" | "lasso">("none");
  const [canvasW, setCanvasW] = useState(900);
  const [canvasH, setCanvasH] = useState(600);
  // Snapshot of the selected text layer's font/color/size so the
  // TextControls picker can render with the real current values
  // instead of snapping back to Inter on re-selection (FU #105).
  const [textProps, setTextProps] = useState<{
    font: string;
    color: string;
    size: number;
  } | null>(null);
  // "auto" = let the router strategy pick. Otherwise a PascalCase Model
  // enum string from the ToolDropdown (or resolved from a `/tool` slug).
  const [model, setModel] = useState<string>("auto");
  const optimize = useOptimizePrompt({
    taskKind: "ImageGeneration",
    value: prompt,
    setValue: setPrompt,
  });
  const notify = useUiStore((s) => s.notify);
  const setActiveGenerate = useAppStore((s) => s.setActiveGenerate);

  // Register generate() as the global Generate handler so the header
  // button fires it. Without this the header's orange button is a no-op
  // on this page. Re-runs when state the handler closes over changes.
  useEffect(() => {
    setActiveGenerate(() => {
      void generate();
    });
    return () => setActiveGenerate(null);
  }, [setActiveGenerate, prompt, model, optimize.enabled]);

  // Toggle selection modes on the canvas whenever the dropdown changes.
  useEffect(() => {
    const handle = canvasRef.current;
    if (!handle) return;
    if (selectionMode === "marquee") handle.enterMarqueeSelect();
    else if (selectionMode === "lasso") handle.enterLassoSelect();
    else handle.exitSelectionMode();
  }, [selectionMode]);

  // Refresh the current-text-props snapshot whenever the selection changes.
  // Runs on selectedId *and* on the layer list in case the underlying object
  // was just added (addText) — the first event fires before the object id
  // is in layersRef.
  useEffect(() => {
    if (!selectedId) {
      setTextProps(null);
      return;
    }
    setTextProps(canvasRef.current?.getTextProperties(selectedId) ?? null);
  }, [selectedId]);

  async function generate() {
    if (!prompt.trim()) return;
    setBusy(true);
    try {
      // Design-spec order (see phase-9 claude-bridge spec):
      //   1) parseOverride on the RAW prompt → strip slug
      //   2) optimize the cleanPrompt (never the slug — Claude must
      //      not see `/flux` or it may mangle the override)
      //   3) re-attach the slug to the optimized text for display so
      //      the user's override survives visually
      //   4) dispatch with the optimized clean text + resolved model
      const parsed = parseOverride(prompt);
      // Fallback when the user typed a slug-only prompt (e.g. just
      // `/flux`): cleanPrompt is empty, so dispatch the raw text so the
      // backend gets *something* rather than an empty-prompt error.
      let textForDispatch = parsed.cleanPrompt || prompt;

      // Only optimize when there's actual content to rewrite — skipping
      // the call for slug-only input avoids asking Claude to expand ""
      // into a meta-prompt.
      if (optimize.enabled && parsed.cleanPrompt) {
        const optimized = await optimize.optimize(parsed.cleanPrompt);
        if (optimized !== undefined) {
          textForDispatch = optimized;
          // Re-attach the slug to the textarea so the user sees their
          // override survived. `optimize()` already wrote the cleaned
          // optimized text via its internal setValue; our second write
          // below is the last one in this sync tick and wins (no flicker).
          if (parsed.override && parsed.slugLocation) {
            const reattached =
              parsed.slugLocation === "start"
                ? `/${parsed.override} ${optimized}`
                : `${optimized} /${parsed.override}`;
            setPrompt(reattached);
          }
        }
      }

      // Slug always beats ToolDropdown; ToolDropdown beats auto-routing.
      const overrideModel = parsed.override ? resolveOverrideToModel(parsed.override) : undefined;
      const finalModel = overrideModel ?? (model === "auto" ? undefined : model);

      const results = await generateVariants({
        prompt: textForDispatch,
        count: 4,
        module: "graphic2d",
        model_override: finalModel,
      });
      setVariants(results);
      notify({
        kind: "success",
        message: `Generated ${results.length} variants`,
      });
    } catch (err) {
      notify({
        kind: "error",
        message: "Generation failed",
        detail: formatError(err),
      });
    } finally {
      setBusy(false);
    }
  }

  async function addVariant(url: string) {
    try {
      await canvasRef.current?.addImageFromUrl(url);
    } catch (err) {
      notify({
        kind: "error",
        message: "Failed to add image",
        detail: formatError(err),
      });
    }
  }

  function addText() {
    canvasRef.current?.addText("Type here");
  }

  function applyFilter(intensity: number) {
    if (!selectedId) return;
    canvasRef.current?.applyFilter(selectedId, filter, intensity);
  }

  function startInpaint() {
    const handle = canvasRef.current;
    if (!handle) return;
    handle.enterMaskMode();
    setMaskMode(true);
  }

  function cancelInpaint() {
    const handle = canvasRef.current;
    if (!handle) return;
    handle.exitMaskMode();
    handle.clearMask();
    setMaskMode(false);
    setInpaintPromptOpen(false);
    setInpaintPrompt("");
  }

  function requestInpaintPrompt() {
    const handle = canvasRef.current;
    if (!handle) return;
    if (!handle.hasMask()) {
      notify({
        kind: "warning",
        message: "Draw a mask first",
        detail: "Paint over the region you want to regenerate.",
      });
      return;
    }
    handle.exitMaskMode();
    setMaskMode(false);
    setInpaintPromptOpen(true);
  }

  async function submitInpaint() {
    const handle = canvasRef.current;
    if (!handle) return;
    const prompt = inpaintPrompt.trim();
    if (!prompt) {
      notify({ kind: "warning", message: "Enter a prompt describing the change" });
      return;
    }
    const sourceUrl = handle.getFirstImageUrl();
    if (!sourceUrl) {
      notify({
        kind: "error",
        message: "No source image",
        detail: "Add an image layer before inpainting.",
      });
      return;
    }
    const maskUrl = handle.getMaskDataUrl();
    // fal.ai flux-fill cannot ingest data-URLs — see pipeline.rs TODO.
    // The mask is produced as a data-URL from the canvas, and in most
    // flows the source image layer is also a remote URL (variant picker)
    // but can be anything. We bail early with a clear message.
    if (isDataUrl(sourceUrl) || isDataUrl(maskUrl)) {
      notify({
        kind: "error",
        message: "Inpainting requires hosted URLs",
        detail:
          "fal.ai's flux-fill endpoint doesn't accept data-URLs yet. Upload pipeline deferred to Phase 5.",
      });
      return;
    }
    setInpaintBusy(true);
    try {
      const result = await inpaintImage({
        prompt,
        source_url: sourceUrl,
        mask_url: maskUrl,
        module: "graphic2d",
      });
      await handle.addImageFromUrl(result.url);
      notify({ kind: "success", message: "Inpaint applied", detail: result.model });
      handle.clearMask();
      setInpaintPromptOpen(false);
      setInpaintPrompt("");
    } catch (err) {
      notify({
        kind: "error",
        message: "Inpaint failed",
        detail: formatError(err),
      });
    } finally {
      setInpaintBusy(false);
    }
  }

  function handleCrop() {
    const handle = canvasRef.current;
    if (!handle) return;
    if (!handle.hasCropSelection()) {
      notify({
        kind: "warning",
        message: "No crop selection",
        detail: "Switch to Marquee or Lasso and draw a region first.",
      });
      return;
    }
    handle.cropToSelection();
    handle.exitSelectionMode();
    setSelectionMode("none");
    // Reflect the new canvas dimensions in the Canvas inputs so the user
    // sees the post-crop size immediately.
    const c = handle.canvas();
    if (c) {
      setCanvasW(c.getWidth());
      setCanvasH(c.getHeight());
    }
  }

  const handleExport = useCallback(async (settings: ExportSettings) => {
    const handle = canvasRef.current;
    if (!handle) return;
    let dataUrl = "";
    switch (settings.format) {
      case "png":
        dataUrl = handle.toPng(settings.transparent);
        break;
      case "jpeg":
        dataUrl = handle.toJpeg(settings.quality);
        break;
      case "webp":
        dataUrl = handle.toWebp(settings.quality);
        break;
      case "svg": {
        const svg = handle.toSvg();
        dataUrl = `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
        break;
      }
      case "pdf":
        dataUrl = handle.toPdf();
        break;
      case "gif":
        dataUrl = await handle.toGif({
          frames: settings.frames,
          delayMs: settings.delayMs,
        });
        break;
    }
    triggerDownload(dataUrl, `${settings.filename}.${extFor(settings.format)}`);
    setExportOpen(false);
  }, []);

  return (
    <div className="grid h-full grid-rows-[auto_1fr]">
      {/* Brief row */}
      <div className="flex flex-col gap-3 border-neutral-dark-700 border-b p-6">
        <div className="flex items-center gap-2">
          <span className="font-mono text-2xs text-accent-500 uppercase tracking-label-wide">
            MOD—02 · 2D GRAPHIC
          </span>
        </div>
        <div className="flex items-center gap-2">
          <ToolDropdown taskKind="ImageGeneration" value={model} onChange={setModel} />
          <OptimizeToggle
            enabled={optimize.enabled}
            onToggle={optimize.setEnabled}
            busy={optimize.busy}
            canUndo={optimize.canUndo}
            onUndo={optimize.undo}
          />
        </div>
        <div className="flex items-end gap-2">
          <div className="flex-1">
            <Input
              label="Describe the image"
              id="graphic2d-prompt"
              placeholder="Minimalist logo for a tech startup, monochrome"
              value={prompt}
              onValueChange={setPrompt}
            />
          </div>
          <LoadingButton
            variant="primary"
            onClick={generate}
            disabled={!prompt.trim()}
            loading={busy}
          >
            <Sparkles className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
            Generate 4 variants
          </LoadingButton>
        </div>

        {busy && variants.length === 0 ? (
          <div className="grid grid-cols-4 gap-2">
            {Array.from({ length: 4 }).map((_, i) => (
              <Skeleton
                // biome-ignore lint/suspicious/noArrayIndexKey: static placeholder list
                key={`skeleton-${i}`}
                className="aspect-square w-full"
              />
            ))}
          </div>
        ) : variants.length > 0 ? (
          <div className="grid grid-cols-4 gap-2">
            {variants.map((v) => (
              <button
                key={v.url}
                type="button"
                onClick={() => addVariant(v.url)}
                className="group relative aspect-square overflow-hidden rounded-xs border border-neutral-dark-600 bg-neutral-dark-800 hover:border-accent-500"
              >
                <img
                  src={v.url}
                  alt=""
                  className="h-full w-full object-cover"
                  onError={(e) => e.currentTarget.replaceWith(fallbackTile(v.url))}
                />
                <span className="pointer-events-none absolute inset-x-0 bottom-0 bg-neutral-dark-950/80 px-2 py-1 text-left font-mono text-2xs text-neutral-dark-200 uppercase tracking-label">
                  {v.model}
                </span>
              </button>
            ))}
          </div>
        ) : null}
      </div>

      {/* Split: toolbar + canvas + layers */}
      <div className="grid min-h-0 grid-cols-[15rem_1fr_14rem]">
        {/* Toolbar */}
        <div className="flex flex-col gap-3 border-neutral-dark-700 border-r p-4">
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            Tools
          </span>
          <Button variant="secondary" size="sm" onClick={addText}>
            <Type className="h-3 w-3" strokeWidth={1.5} />
            Add text
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => variants[0] && addVariant(variants[0].url)}
            disabled={variants.length === 0}
          >
            <ImageIcon className="h-3 w-3" strokeWidth={1.5} />
            Add first variant
          </Button>

          <div className="mt-2 flex flex-col gap-2">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Inpaint
            </span>
            {maskMode ? (
              <div className="flex gap-1">
                <Button
                  variant="primary"
                  size="sm"
                  onClick={requestInpaintPrompt}
                  disabled={inpaintBusy}
                >
                  Apply
                </Button>
                <Button variant="secondary" size="sm" onClick={cancelInpaint}>
                  Cancel
                </Button>
              </div>
            ) : (
              <Button
                variant="secondary"
                size="sm"
                onClick={startInpaint}
                disabled={layers.length === 0 || inpaintBusy}
                aria-label="Start inpaint: draw mask"
              >
                <Brush className="h-3 w-3" strokeWidth={1.5} />
                Draw mask
              </Button>
            )}
          </div>

          {layers.find((l) => l.id === selectedId)?.type === "text" && textProps ? (
            <div className="mt-2">
              <TextControls
                // key={selectedId} — remount on selection change so local
                // picker state starts fresh, now hydrated from the real
                // Textbox state via textProps (FU #105).
                key={selectedId}
                initialFont={textProps.font}
                initialColor={textProps.color}
                initialSize={textProps.size}
                onChange={(patch) => {
                  if (selectedId) canvasRef.current?.updateText(selectedId, patch);
                }}
              />
            </div>
          ) : null}

          <div className="mt-2 flex flex-col gap-2">
            <span className="inline-flex items-center gap-1.5 font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Filter
              <HelpIcon content="Select an image layer, then drag to apply. Negative values soften, positive values strengthen the effect." />
            </span>
            <Dropdown
              value={filter}
              onChange={(v) => setFilter(v as typeof filter)}
              options={[
                { value: "blur", label: "Blur" },
                { value: "sharpen", label: "Sharpen" },
                { value: "brightness", label: "Brightness" },
                { value: "contrast", label: "Contrast" },
                { value: "saturation", label: "Saturation" },
              ]}
            />
            <input
              type="range"
              min={-1}
              max={1}
              step={0.05}
              defaultValue={0}
              onChange={(e) => applyFilter(Number(e.currentTarget.value))}
              className="w-full accent-accent-500"
              disabled={!selectedId}
            />
          </div>

          <div className="mt-2 flex flex-col gap-1">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Transform
            </span>
            <div className="flex gap-1">
              <Button
                variant="secondary"
                size="sm"
                onClick={() => selectedId && canvasRef.current?.flipH(selectedId)}
                disabled={!selectedId}
              >
                Flip H
              </Button>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => selectedId && canvasRef.current?.flipV(selectedId)}
                disabled={!selectedId}
              >
                Flip V
              </Button>
            </div>
          </div>

          <div className="mt-2 flex flex-col gap-1">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Selection
            </span>
            <Dropdown
              value={selectionMode}
              onChange={(v) => setSelectionMode(v as typeof selectionMode)}
              options={[
                { value: "none", label: "Off" },
                { value: "marquee", label: "Marquee" },
                { value: "lasso", label: "Lasso" },
              ]}
            />
            <Button variant="secondary" size="sm" onClick={handleCrop}>
              Crop to selection
            </Button>
          </div>

          <div className="mt-2 flex flex-col gap-1">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Canvas
            </span>
            <div className="flex gap-1">
              <Input
                type="number"
                label="W"
                id="canvas-w"
                value={canvasW}
                onValueChange={(v) => setCanvasW(Number(v))}
              />
              <Input
                type="number"
                label="H"
                id="canvas-h"
                value={canvasH}
                onValueChange={(v) => setCanvasH(Number(v))}
              />
            </div>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => canvasRef.current?.setCanvasSize(canvasW, canvasH)}
            >
              Resize
            </Button>
          </div>

          <Button
            variant="primary"
            size="sm"
            className="mt-auto"
            onClick={() => setExportOpen(true)}
            disabled={layers.length === 0}
          >
            <Download className="h-3 w-3" strokeWidth={1.5} />
            Export
          </Button>
        </div>

        {/* Canvas */}
        <FabricCanvas
          ref={canvasRef}
          width={900}
          height={600}
          onLayersChange={setLayers}
          onSelectionChange={setSelectedId}
        />

        {/* Layers */}
        <div className="flex flex-col border-neutral-dark-700 border-l">
          <div className="flex items-center justify-between border-neutral-dark-700 border-b px-3 py-2">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Layers · {layers.length}
            </span>
            <button
              type="button"
              onClick={addText}
              aria-label="Add text layer"
              className="text-neutral-dark-400 hover:text-neutral-dark-100"
            >
              <Plus className="h-3 w-3" strokeWidth={1.5} />
            </button>
          </div>
          <div className="flex-1 overflow-y-auto">
            <LayerList
              layers={layers}
              selectedId={selectedId}
              onSelect={(id) => {
                setSelectedId(id);
              }}
              onToggleVisible={(id) => canvasRef.current?.toggleVisibility(id)}
              onToggleLock={(id) => canvasRef.current?.toggleLock(id)}
              onRemove={(id) => canvasRef.current?.removeLayer(id)}
            />
          </div>
        </div>
      </div>

      <ExportDialog
        open={exportOpen}
        onClose={() => setExportOpen(false)}
        onExport={handleExport}
      />

      {inpaintPromptOpen ? (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-neutral-dark-950/80 p-4">
          <div className="w-full max-w-md rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 p-5">
            <div className="mb-4 flex items-center gap-2">
              <Brush className="h-3 w-3 text-accent-500" strokeWidth={1.5} />
              <span className="font-mono text-2xs text-accent-500 uppercase tracking-label-wide">
                Inpaint · Describe change
              </span>
            </div>
            <Input
              id="inpaint-prompt"
              label="Prompt"
              placeholder="replace with flowers, same lighting"
              value={inpaintPrompt}
              onValueChange={setInpaintPrompt}
              autoFocus
            />
            <div className="mt-4 flex justify-end gap-2">
              <Button variant="secondary" size="sm" onClick={cancelInpaint} disabled={inpaintBusy}>
                Cancel
              </Button>
              <LoadingButton
                variant="primary"
                size="sm"
                onClick={submitInpaint}
                disabled={!inpaintPrompt.trim()}
                loading={inpaintBusy}
              >
                Apply inpaint
              </LoadingButton>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}

// ─── Helpers ────────────────────────────────────────────────────────────

function extFor(f: ExportFormat): string {
  switch (f) {
    case "jpeg":
      return "jpg";
    default:
      return f;
  }
}

function triggerDownload(dataUrl: string, filename: string) {
  const a = document.createElement("a");
  a.href = dataUrl;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
}

/** Fallback for stub:// variant URLs that won't load in an <img>. */
function fallbackTile(url: string): HTMLElement {
  const div = document.createElement("div");
  div.className =
    "flex h-full w-full items-center justify-center bg-neutral-dark-800 p-2 text-center font-mono text-2xs text-neutral-dark-400 tracking-label uppercase";
  div.textContent = `stub · ${url.split("/").pop() ?? ""}`;
  return div;
}
