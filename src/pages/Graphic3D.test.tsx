import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

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

// GltfModel uses drei's useGLTF which isn't stubbed; the mesh scene would
// try to load the GLB the moment generateMeshFromText resolves. Stub the
// component out so the page renders the mesh path without WebGL.
vi.mock("@/components/graphic3d/GltfModel", () => ({
  GltfModel: () => null,
}));

vi.mock("@/lib/meshCommands", () => ({
  generateMeshFromText: vi.fn(async () => ({
    glb_url: "https://fake/mesh.glb",
    local_path: null,
    model: "MeshyText3D",
  })),
  generateMeshFromImage: vi.fn(),
  exportMesh: vi.fn(),
}));

vi.mock("@/lib/depthCommands", () => ({
  generateDepth: vi.fn(),
}));

import { generateMeshFromText } from "@/lib/meshCommands";
import { Graphic3DPage } from "@/pages/Graphic3D";

describe("Graphic3DPage", () => {
  beforeEach(() => {
    vi.mocked(generateMeshFromText).mockClear();
  });

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

  it("parses `/meshy a dragon` prompt: model_override=MeshyText3D, cleanPrompt=a dragon", async () => {
    render(
      <MemoryRouter>
        <Graphic3DPage />
      </MemoryRouter>,
    );
    fireEvent.change(screen.getByLabelText(/describe a 3d object/i), {
      target: { value: "/meshy a dragon" },
    });
    fireEvent.click(screen.getByRole("button", { name: /^generate 3d$/i }));
    await waitFor(() => expect(generateMeshFromText).toHaveBeenCalledTimes(1));
    expect(vi.mocked(generateMeshFromText).mock.calls[0]?.[0]).toMatchObject({
      prompt: "a dragon",
      module: "graphic3d",
      model_override: "MeshyText3D",
    });
  });
});
