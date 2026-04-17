import "@testing-library/jest-dom/vitest";
// vitest-canvas-mock patches HTMLCanvasElement.getContext() so libraries like
// Fabric.js that instantiate a 2D context during construction can run under
// jsdom. Required for FabricCanvas.test.tsx (FU #104).
import "vitest-canvas-mock";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

afterEach(() => {
  cleanup();
});
