import { useState } from "react";
import { CameraControls, type CameraMode } from "@/components/graphic3d/CameraControls";
import { ThreeCanvas } from "@/components/graphic3d/ThreeCanvas";

export function Graphic3DPage() {
  const [cameraMode, setCameraMode] = useState<CameraMode>("perspective");

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
        </div>
        <ThreeCanvas cameraMode={cameraMode}>
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
