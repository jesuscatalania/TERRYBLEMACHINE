import { OrbitControls } from "@react-three/drei";
import { Canvas } from "@react-three/fiber";
import type { ReactNode } from "react";

export interface ThreeCanvasProps {
  children?: ReactNode;
  className?: string;
}

export function ThreeCanvas({ children, className }: ThreeCanvasProps) {
  return (
    <div className={`relative h-full w-full bg-neutral-dark-950 ${className ?? ""}`}>
      <Canvas camera={{ position: [4, 3, 4], fov: 45 }} dpr={[1, 2]}>
        <ambientLight intensity={0.5} />
        <directionalLight position={[5, 5, 5]} intensity={1} />
        <OrbitControls makeDefault />
        {children}
      </Canvas>
    </div>
  );
}
