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

/** Stand-ins for @react-three/drei exports used across the app. */
export const DreiStubs = {
  OrbitControls: () => null,
  Environment: () => null,
  Stats: () => null,
};
