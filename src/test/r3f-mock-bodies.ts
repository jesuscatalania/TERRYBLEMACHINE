// Shared stub bodies for @react-three/fiber + @react-three/drei under jsdom.
// jsdom lacks WebGL/ResizeObserver, so rendering <Canvas> crashes. Test files
// still call vi.mock(...) themselves (vi.mock hoisting is per-file), but the
// stub implementations live here so drei additions (PerspectiveCamera, Bloom,
// etc.) only need to be maintained in one place.

import { createElement, type ReactNode } from "react";

/** Stand-in for R3F's <Canvas>. Renders children in a plain <div>. */
export function FiberCanvasStub(props: { children?: ReactNode }) {
  return createElement("div", { "data-testid": "three-canvas" }, props.children);
}

/**
 * Stand-in for R3F's `useThree` hook. The real hook reads the fiber store
 * set up by <Canvas>; under the FiberCanvasStub there is no such store, so
 * consumers (ExportHandle, etc.) get a shape-compatible fake with just the
 * pieces we currently touch. Extend as new consumers appear.
 */
export function useThreeStub() {
  const canvas = document.createElement("canvas");
  const gl = {
    domElement: canvas,
    render: () => {},
  };
  const scene = {};
  const camera = {
    position: {
      x: 4,
      y: 3,
      z: 4,
      set: () => {},
      clone: () => ({ x: 4, y: 3, z: 4 }),
    },
    lookAt: () => {},
  };
  return { gl, scene, camera };
}

/** Stand-ins for @react-three/drei exports used across the app. */
export const DreiStubs = {
  OrbitControls: () => null,
  Environment: () => null,
  Stats: () => null,
};
