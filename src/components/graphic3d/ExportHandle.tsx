import { useThree } from "@react-three/fiber";
import { type MutableRefObject, useEffect } from "react";
import type { Camera, Scene, WebGLRenderer } from "three";

/**
 * Bundle of three.js objects required to render a frame out-of-tree. Emitted
 * by <ExportHandle /> into a ref so the parent page can call `gl.render(scene,
 * camera)` — the minimum needed to orbit the camera and snapshot frames for
 * an animated GIF (see `captureAnimatedGif`).
 */
export interface ExportRefs {
  gl: WebGLRenderer | null;
  scene: Scene | null;
  camera: Camera | null;
}

export interface ExportHandleProps {
  handleRef: MutableRefObject<ExportRefs | null>;
}

/**
 * Invisible child component intended to be rendered inside an R3F <Canvas>.
 * Captures the active WebGLRenderer, Scene, and Camera into the provided ref
 * so out-of-tree consumers (e.g., an export dialog in the parent page) can
 * call `gl.domElement.toDataURL(...)` on the current frame or orbit the
 * camera and re-render via `gl.render(scene, camera)` for animated exports.
 *
 * Requires `preserveDrawingBuffer: true` on the Canvas for `toDataURL` to
 * return a valid image after R3F's render-on-demand cycle.
 */
export function ExportHandle({ handleRef }: ExportHandleProps) {
  const { gl, scene, camera } = useThree();
  useEffect(() => {
    handleRef.current = { gl, scene, camera };
    return () => {
      handleRef.current = null;
    };
  }, [gl, scene, camera, handleRef]);
  return null;
}
