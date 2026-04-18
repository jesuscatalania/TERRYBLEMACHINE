import { OrbitControls } from "@react-three/drei";
import { Canvas } from "@react-three/fiber";
import type { MutableRefObject, ReactNode } from "react";
import type { WebGLRenderer } from "three";
import type { CameraMode } from "./CameraControls";
import { ExportHandle } from "./ExportHandle";
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
  /**
   * Optional ref that will receive the active WebGLRenderer once the Canvas
   * has mounted. Used by the parent page to capture the current frame for
   * image export (PNG/JPEG/WebP/PDF). When provided, an <ExportHandle /> is
   * mounted inside the Canvas to wire the ref via useThree.
   */
  glRef?: MutableRefObject<WebGLRenderer | null>;
}

export function ThreeCanvas({
  children,
  className,
  cameraMode = "perspective",
  lighting = "studio",
  bloom,
  ssao,
  isoPreset = "none",
  glRef,
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
      <Canvas
        key={`${cameraMode}-${lighting}-${isoPreset}`}
        {...canvasProps}
        dpr={[1, 2]}
        gl={{ preserveDrawingBuffer: true, alpha: true }}
      >
        <LightingPreset name={lighting} />
        <OrbitControls makeDefault />
        {children}
        <PostProcessing bloom={bloom} ssao={ssao} />
        {glRef ? <ExportHandle handleRef={glRef} /> : null}
      </Canvas>
    </div>
  );
}
