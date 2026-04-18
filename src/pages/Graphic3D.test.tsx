import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

// R3F uses WebGL which jsdom cannot provide. Stub the <Canvas> wrapper so
// the page-level test verifies the shell + controls, not the WebGL tree.
// NOTE: vi.mock is hoisted above imports, so stubs must be imported *inside*
// the factory — see src/test/r3f-mock-bodies.ts.
vi.mock("@react-three/fiber", async () => {
  const [actual, { FiberCanvasStub, useThreeStub }] = await Promise.all([
    vi.importActual<typeof import("@react-three/fiber")>("@react-three/fiber"),
    import("@/test/r3f-mock-bodies"),
  ]);
  return { ...actual, Canvas: FiberCanvasStub, useThree: useThreeStub };
});

vi.mock("@react-three/drei", async () => {
  const { DreiStubs } = await import("@/test/r3f-mock-bodies");
  return DreiStubs;
});

import { Graphic3DPage } from "@/pages/Graphic3D";

describe("Graphic3DPage", () => {
  it("renders the module banner", () => {
    render(
      <MemoryRouter>
        <Graphic3DPage />
      </MemoryRouter>,
    );
    expect(screen.getByText(/MOD—03/)).toBeInTheDocument();
    expect(screen.getByText(/PSEUDO-3D/i)).toBeInTheDocument();
  });

  it("mounts a Three canvas", () => {
    render(
      <MemoryRouter>
        <Graphic3DPage />
      </MemoryRouter>,
    );
    expect(screen.getByTestId("three-canvas")).toBeInTheDocument();
  });
});
