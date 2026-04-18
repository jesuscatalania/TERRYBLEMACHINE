import { OrbitControls } from "@react-three/drei";
import { Canvas } from "@react-three/fiber";
import type { ReactNode } from "react";
import type { CameraMode } from "./CameraControls";
import { cameraForIso, type IsoPresetName } from "./IsoPreset";
import { type LightingName, LightingPreset } from "./LightingPreset";
import { PostProcessing } from "./PostProcessing";

export interface ThreeCanvasProps {
  children?: ReactNode;
  className?: string;
  cameraMode?: CameraMode;
  lighting?: LightingName;
  bloom?: boolean;
  ssao?: boolean;
  isoPreset?: IsoPresetName;
}

export function ThreeCanvas({
  children,
  className,
  cameraMode = "perspective",
  lighting = "studio",
  bloom,
  ssao,
  isoPreset = "none",
}: ThreeCanvasProps) {
  const iso = cameraForIso(isoPreset);
  const defaultPosition: [number, number, number] = [4, 3, 4];
  const position = iso?.position ?? defaultPosition;
  const fov = iso?.fov ?? 45;

  const canvasProps =
    cameraMode === "orthographic"
      ? {
          orthographic: true as const,
          camera: {
            position,
            zoom: 100,
            near: 0.1,
            far: 1000,
          },
        }
      : {
          camera: {
            position,
            fov,
          },
        };

  return (
    <div className={`relative h-full w-full bg-neutral-dark-950 ${className ?? ""}`}>
      <Canvas key={`${cameraMode}-${lighting}-${isoPreset}`} {...canvasProps} dpr={[1, 2]}>
        <LightingPreset name={lighting} />
        <OrbitControls makeDefault />
        {children}
        <PostProcessing bloom={bloom} ssao={ssao} />
      </Canvas>
    </div>
  );
}
