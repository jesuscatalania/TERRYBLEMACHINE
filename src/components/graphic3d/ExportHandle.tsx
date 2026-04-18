import { useThree } from "@react-three/fiber";
import { type MutableRefObject, useEffect } from "react";
import type { WebGLRenderer } from "three";

export interface ExportHandleProps {
  handleRef: MutableRefObject<WebGLRenderer | null>;
}

/**
 * Invisible child component intended to be rendered inside an R3F <Canvas>.
 * Captures the active WebGLRenderer into the provided ref so out-of-tree
 * consumers (e.g., an export dialog in the parent page) can call
 * `gl.domElement.toDataURL(...)` on the current frame.
 *
 * Requires `preserveDrawingBuffer: true` on the Canvas for `toDataURL` to
 * return a valid image after R3F's render-on-demand cycle.
 */
export function ExportHandle({ handleRef }: ExportHandleProps) {
  const { gl } = useThree();
  useEffect(() => {
    handleRef.current = gl;
    return () => {
      handleRef.current = null;
    };
  }, [gl, handleRef]);
  return null;
}
