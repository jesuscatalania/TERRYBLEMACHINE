import { jsPDF } from "jspdf";
import { Download } from "lucide-react";
import { Suspense, useCallback, useRef, useState } from "react";
import type { WebGLRenderer } from "three";
import { CameraControls, type CameraMode } from "@/components/graphic3d/CameraControls";
import { DepthPlane } from "@/components/graphic3d/DepthPlane";
import { GltfModel } from "@/components/graphic3d/GltfModel";
import type { IsoPresetName } from "@/components/graphic3d/IsoPreset";
import type { LightingName } from "@/components/graphic3d/LightingPreset";
import { ThreeCanvas } from "@/components/graphic3d/ThreeCanvas";
import {
  ThreeExportDialog,
  type ThreeExportSettings,
} from "@/components/graphic3d/ThreeExportDialog";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { type DepthResult, generateDepth } from "@/lib/depthCommands";
import {
  exportMesh,
  generateMeshFromImage,
  generateMeshFromText,
  type MeshResult,
} from "@/lib/meshCommands";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";

export function Graphic3DPage() {
  const [cameraMode, setCameraMode] = useState<CameraMode>("perspective");
  const [lighting, setLighting] = useState<LightingName>("studio");
  const [isoPreset, setIsoPreset] = useState<IsoPresetName>("none");
  const [bloom, setBloom] = useState(false);
  const [ssao, setSsao] = useState(false);
  const [imageUrl, setImageUrl] = useState("");
  const [depthResult, setDepthResult] = useState<DepthResult | null>(null);
  const [depthBusy, setDepthBusy] = useState(false);
  const [displacementScale, setDisplacementScale] = useState(0.5);
  const [meshPrompt, setMeshPrompt] = useState("");
  const [meshResult, setMeshResult] = useState<MeshResult | null>(null);
  const [meshBusy, setMeshBusy] = useState(false);
  const [imageMeshBusy, setImageMeshBusy] = useState(false);
  const [quickPreview, setQuickPreview] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
  const glRef = useRef<WebGLRenderer | null>(null);
  const notify = useUiStore((s) => s.notify);
  const currentProject = useProjectStore((s) => s.currentProject);

  async function generateDepthForImage() {
    const trimmed = imageUrl.trim();
    if (!trimmed) return;
    setDepthBusy(true);
    try {
      const result = await generateDepth({ image_url: trimmed, module: "graphic3d" });
      setDepthResult(result);
      notify({ kind: "success", message: "Depth map ready", detail: result.model });
    } catch (err) {
      notify({
        kind: "error",
        message: "Depth generation failed",
        detail: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setDepthBusy(false);
    }
  }

  async function generate3D() {
    const trimmed = meshPrompt.trim();
    if (!trimmed) return;
    setMeshBusy(true);
    try {
      const result = await generateMeshFromText({ prompt: trimmed, module: "graphic3d" });
      setMeshResult(result);
      notify({ kind: "success", message: "3D mesh ready", detail: result.model });
    } catch (err) {
      notify({
        kind: "error",
        message: "3D generation failed",
        detail: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setMeshBusy(false);
    }
  }

  async function generate3DFromImage() {
    const trimmed = imageUrl.trim();
    if (!trimmed) return;
    setImageMeshBusy(true);
    try {
      const result = await generateMeshFromImage({
        image_url: trimmed,
        module: "graphic3d",
        quick_preview: quickPreview,
      });
      setMeshResult(result);
      notify({
        kind: "success",
        message: quickPreview ? "Quick mesh ready" : "Mesh ready",
        detail: result.model,
      });
    } catch (err) {
      notify({
        kind: "error",
        message: "Image-to-3D failed",
        detail: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setImageMeshBusy(false);
    }
  }

  async function exportGlb() {
    if (!meshResult?.local_path) return;
    if (!currentProject) {
      notify({
        kind: "warning",
        message: "Open a project first to export",
      });
      return;
    }
    const filename = `${Date.now()}-mesh.glb`;
    const targetPath = `${currentProject.path}/exports/${filename}`;
    try {
      const saved = await exportMesh(meshResult.local_path, targetPath);
      notify({ kind: "success", message: "GLB exported", detail: saved });
    } catch (err) {
      notify({
        kind: "error",
        message: "GLB export failed",
        detail: err instanceof Error ? err.message : String(err),
      });
    }
  }

  const handleExport = useCallback(
    (settings: ThreeExportSettings) => {
      const dataUrl = captureFrame(glRef.current, settings);
      if (!dataUrl) {
        notify({
          kind: "error",
          message: "Canvas not ready",
          detail: "Try rotating the scene first.",
        });
        return;
      }
      const ext = settings.format === "jpeg" ? "jpg" : settings.format;
      triggerDownload(dataUrl, `${settings.filename}.${ext}`);
      setExportOpen(false);
    },
    [notify],
  );

  return (
    <div className="grid h-full grid-rows-[auto_1fr]">
      <div className="flex flex-col gap-3 border-neutral-dark-700 border-b p-6">
        <div className="flex items-center gap-2">
          <span className="font-mono text-2xs text-accent-500 uppercase tracking-label-wide">
            MOD—03 · PSEUDO-3D
          </span>
        </div>
        <div className="flex items-end gap-2">
          <div className="flex-1">
            <Input
              label="Source image URL (for depth)"
              id="graphic3d-image-url"
              placeholder="https://example.com/image.png"
              value={imageUrl}
              onValueChange={(value) => {
                setImageUrl(value);
                if (depthResult) setDepthResult(null);
              }}
            />
          </div>
          <Button
            variant="secondary"
            onClick={generateDepthForImage}
            disabled={!imageUrl.trim() || depthBusy}
          >
            {depthBusy ? "Generating…" : "Generate depth"}
          </Button>
        </div>
        {depthBusy ? (
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            Requesting depth map…
          </span>
        ) : null}
        <div className="flex items-end gap-2">
          <Button
            variant="secondary"
            onClick={generate3DFromImage}
            disabled={!imageUrl.trim() || imageMeshBusy}
          >
            {imageMeshBusy ? "Generating…" : "Generate 3D from image"}
          </Button>
          <label className="flex items-center gap-2 text-2xs text-neutral-dark-300">
            <input
              type="checkbox"
              checked={quickPreview}
              onChange={(e) => setQuickPreview(e.target.checked)}
              className="accent-accent-500"
            />
            Quick preview (TripoSR)
          </label>
        </div>
        {imageMeshBusy ? (
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            Requesting image-to-3D…
          </span>
        ) : null}
        <div className="flex items-end gap-2">
          <div className="flex-1">
            <Input
              label="Describe a 3D object (Meshy)"
              id="graphic3d-mesh-prompt"
              placeholder="a minimalist wooden desk"
              value={meshPrompt}
              onValueChange={setMeshPrompt}
            />
          </div>
          <Button
            variant="secondary"
            onClick={generate3D}
            disabled={!meshPrompt.trim() || meshBusy}
          >
            {meshBusy ? "Generating…" : "Generate 3D"}
          </Button>
        </div>
        {meshBusy ? (
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            Requesting 3D mesh…
          </span>
        ) : null}
      </div>
      <div className="grid min-h-0 grid-cols-[15rem_1fr_14rem]">
        <div className="flex flex-col gap-3 border-neutral-dark-700 border-r p-4">
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            Tools
          </span>
          <CameraControls mode={cameraMode} onModeChange={setCameraMode} />
          <div className="flex flex-col gap-1">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Lighting
            </span>
            <select
              value={lighting}
              onChange={(e) => setLighting(e.target.value as LightingName)}
              className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 text-xs text-neutral-dark-100"
            >
              <option value="studio">Studio</option>
              <option value="outdoor">Outdoor</option>
              <option value="dramatic">Dramatic</option>
            </select>
          </div>
          <div className="flex flex-col gap-1">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Preset
            </span>
            <select
              value={isoPreset}
              onChange={(e) => setIsoPreset(e.target.value as IsoPresetName)}
              className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 text-xs text-neutral-dark-100"
            >
              <option value="none">None</option>
              <option value="room">Room</option>
              <option value="city">City Block</option>
              <option value="product">Product Shot</option>
            </select>
          </div>
          <div className="mt-2 flex flex-col gap-1">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Post-FX
            </span>
            <label className="flex items-center gap-2 text-neutral-dark-200 text-xs">
              <input
                type="checkbox"
                checked={bloom}
                onChange={(e) => setBloom(e.target.checked)}
                className="accent-accent-500"
              />
              Bloom
            </label>
            <label className="flex items-center gap-2 text-neutral-dark-200 text-xs">
              <input
                type="checkbox"
                checked={ssao}
                onChange={(e) => setSsao(e.target.checked)}
                className="accent-accent-500"
              />
              SSAO
            </label>
          </div>
          <Button
            variant="primary"
            size="sm"
            className="mt-auto"
            onClick={() => setExportOpen(true)}
          >
            <Download className="h-3 w-3" strokeWidth={1.5} />
            Export
          </Button>
        </div>
        <ThreeCanvas
          cameraMode={cameraMode}
          lighting={lighting}
          bloom={bloom}
          ssao={ssao}
          isoPreset={isoPreset}
          glRef={glRef}
        >
          {meshResult ? (
            <Suspense fallback={null}>
              <GltfModel localPath={meshResult.local_path} remoteUrl={meshResult.glb_url} />
            </Suspense>
          ) : depthResult ? (
            <DepthPlane
              imageUrl={imageUrl}
              depthUrl={depthResult.depth_url}
              displacementScale={displacementScale}
            />
          ) : (
            <mesh>
              <boxGeometry args={[1, 1, 1]} />
              <meshStandardMaterial color="#e85d2d" />
            </mesh>
          )}
        </ThreeCanvas>
        <div className="flex flex-col border-neutral-dark-700 border-l">
          <div className="flex items-center justify-between border-neutral-dark-700 border-b px-3 py-2">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Scene
            </span>
          </div>
          <div className="flex-1 overflow-y-auto">
            {meshResult ? (
              <div className="flex flex-col gap-2 p-3">
                <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
                  Mesh
                </span>
                <span className="font-mono text-2xs text-neutral-dark-500">{meshResult.model}</span>
                <span
                  className="truncate font-mono text-2xs text-neutral-dark-600"
                  title={meshResult.local_path ?? meshResult.glb_url}
                >
                  {meshResult.local_path ? "cached locally" : "remote URL"}
                </span>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={exportGlb}
                  disabled={!meshResult.local_path}
                >
                  Export GLB
                </Button>
              </div>
            ) : depthResult ? (
              <div className="flex flex-col gap-2 p-3">
                <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
                  Depth
                </span>
                <span className="font-mono text-2xs text-neutral-dark-500">
                  {depthResult.model}
                </span>
                <label className="flex flex-col gap-1 text-2xs text-neutral-dark-200">
                  Displacement: {displacementScale.toFixed(2)}
                  <input
                    type="range"
                    min={0}
                    max={2}
                    step={0.05}
                    value={displacementScale}
                    onChange={(e) => setDisplacementScale(Number(e.target.value))}
                    className="accent-accent-500"
                  />
                </label>
              </div>
            ) : (
              <div className="p-3">
                <span className="font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
                  Empty
                </span>
              </div>
            )}
          </div>
        </div>
      </div>
      <ThreeExportDialog
        open={exportOpen}
        onClose={() => setExportOpen(false)}
        onExport={handleExport}
      />
    </div>
  );
}

// ─── Helpers ────────────────────────────────────────────────────────────

/**
 * Read the current WebGL frame out of the renderer's canvas and encode it in
 * the requested format. Relies on `preserveDrawingBuffer: true` being set on
 * the Canvas so the last-rendered frame is still in the buffer when we call
 * `toDataURL`. Returns an empty string if the ref is unset (Canvas not ready).
 */
function captureFrame(gl: WebGLRenderer | null, settings: ThreeExportSettings): string {
  if (!gl) return "";
  const canvas = gl.domElement;

  if (settings.format === "pdf") {
    const png = canvas.toDataURL("image/png");
    const pdf = new jsPDF({
      orientation: canvas.width >= canvas.height ? "landscape" : "portrait",
      unit: "px",
      format: [canvas.width, canvas.height],
    });
    pdf.addImage(png, "PNG", 0, 0, canvas.width, canvas.height);
    return pdf.output("dataurlstring");
  }

  // PNG with transparency flows through toDataURL directly — the Canvas is
  // configured with alpha:true so the buffer already has per-pixel alpha.
  // For opaque PNG we composite onto black via an offscreen 2D canvas so the
  // exported PNG matches what the user sees (dark background).
  if (settings.format === "png" && !settings.transparent) {
    const off = document.createElement("canvas");
    off.width = canvas.width;
    off.height = canvas.height;
    const ctx = off.getContext("2d");
    if (!ctx) return canvas.toDataURL("image/png");
    ctx.fillStyle = "#000";
    ctx.fillRect(0, 0, off.width, off.height);
    ctx.drawImage(canvas, 0, 0);
    return off.toDataURL("image/png");
  }

  const mime = `image/${settings.format}` as const;
  return canvas.toDataURL(mime, settings.quality);
}

function triggerDownload(dataUrl: string, filename: string) {
  const a = document.createElement("a");
  a.href = dataUrl;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
}
