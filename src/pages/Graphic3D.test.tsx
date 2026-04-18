import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

// R3F uses WebGL which jsdom cannot provide. Stub the <Canvas> wrapper so
// the page-level test verifies the shell + controls, not the WebGL tree.
vi.mock("@react-three/fiber", async () => {
  const actual = await vi.importActual<typeof import("@react-three/fiber")>("@react-three/fiber");
  return {
    ...actual,
    Canvas: (props: { children?: ReactNode }) => (
      <div data-testid="three-canvas">{props.children}</div>
    ),
  };
});

vi.mock("@react-three/drei", () => ({
  OrbitControls: () => null,
  Environment: () => null,
}));

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
