import { OrbitControls } from "@react-three/drei";
import { Canvas } from "@react-three/fiber";
import type { ReactNode } from "react";
import type { CameraMode } from "./CameraControls";

export interface ThreeCanvasProps {
  children?: ReactNode;
  className?: string;
  cameraMode?: CameraMode;
}

export function ThreeCanvas({ children, className, cameraMode = "perspective" }: ThreeCanvasProps) {
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
      <Canvas key={cameraMode} {...canvasProps} dpr={[1, 2]}>
        <ambientLight intensity={0.5} />
        <directionalLight position={[5, 5, 5]} intensity={1} />
        <OrbitControls makeDefault />
        {children}
      </Canvas>
    </div>
  );
}
