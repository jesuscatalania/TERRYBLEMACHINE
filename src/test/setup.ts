import "@testing-library/jest-dom/vitest";
// vitest-canvas-mock patches HTMLCanvasElement.getContext() so libraries like
// Fabric.js that instantiate a 2D context during construction can run under
// jsdom. Required for FabricCanvas.test.tsx (FU #104).
import "vitest-canvas-mock";
import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach } from "vitest";
import { WELCOME_LOCALSTORAGE_KEY } from "@/hooks/useWelcomeFlow";

afterEach(() => {
  cleanup();
});

// Pre-dismiss the Welcome modal in unit tests so it doesn't sit on top of
// every render. E2E tests clear this flag explicitly when they want to
// exercise the onboarding flow.
beforeEach(() => {
  window.localStorage.setItem(WELCOME_LOCALSTORAGE_KEY, "true");
});
