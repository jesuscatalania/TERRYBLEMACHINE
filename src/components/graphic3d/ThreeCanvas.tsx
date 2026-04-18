import { OrbitControls } from "@react-three/drei";
import { Canvas } from "@react-three/fiber";
import type { ReactNode } from "react";
import type { CameraMode } from "./CameraControls";
import { type LightingName, LightingPreset } from "./LightingPreset";
import { PostProcessing } from "./PostProcessing";

export interface ThreeCanvasProps {
  children?: ReactNode;
  className?: string;
  cameraMode?: CameraMode;
  lighting?: LightingName;
  bloom?: boolean;
  ssao?: boolean;
}

export function ThreeCanvas({
  children,
  className,
  cameraMode = "perspective",
  lighting = "studio",
  bloom,
  ssao,
}: ThreeCanvasProps) {
  const canvasProps =
    cameraMode === "orthographic"
      ? {
          orthographic: true as const,
          camera: {
            position: [4, 3, 4] as [number, number, number],
            zoom: 100,
            near: 0.1,
            far: 1000,
          },
        }
      : {
          camera: {
            position: [4, 3, 4] as [number, number, number],
            fov: 45,
          },
        };

  return (
    <div className={`relative h-full w-full bg-neutral-dark-950 ${className ?? ""}`}>
      <Canvas key={`${cameraMode}-${lighting}`} {...canvasProps} dpr={[1, 2]}>
        <LightingPreset name={lighting} />
        <OrbitControls makeDefault />
        {children}
        <PostProcessing bloom={bloom} ssao={ssao} />
      </Canvas>
    </div>
  );
}
