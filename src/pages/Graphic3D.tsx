import { useState } from "react";
import { CameraControls, type CameraMode } from "@/components/graphic3d/CameraControls";
import type { LightingName } from "@/components/graphic3d/LightingPreset";
import { ThreeCanvas } from "@/components/graphic3d/ThreeCanvas";

export function Graphic3DPage() {
  const [cameraMode, setCameraMode] = useState<CameraMode>("perspective");
  const [lighting, setLighting] = useState<LightingName>("studio");
  const [bloom, setBloom] = useState(false);
  const [ssao, setSsao] = useState(false);

  return (
    <div className="grid h-full grid-rows-[auto_1fr]">
      <div className="flex flex-col gap-3 border-neutral-dark-700 border-b p-6">
        <div className="flex items-center gap-2">
          <span className="font-mono text-2xs text-accent-500 uppercase tracking-label-wide">
            MOD—03 · PSEUDO-3D
          </span>
        </div>
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
        </div>
        <ThreeCanvas cameraMode={cameraMode} lighting={lighting} bloom={bloom} ssao={ssao}>
          <mesh>
            <boxGeometry args={[1, 1, 1]} />
            <meshStandardMaterial color="#e85d2d" />
          </mesh>
        </ThreeCanvas>
        <div className="flex flex-col border-neutral-dark-700 border-l">
          <div className="flex items-center justify-between border-neutral-dark-700 border-b px-3 py-2">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Scene
            </span>
          </div>
          <div className="flex-1 overflow-y-auto p-3">
            <span className="font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
              Empty
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
