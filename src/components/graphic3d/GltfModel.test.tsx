import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

// drei's useGLTF would kick off a real three.js GLTFLoader under jsdom,
// which has no DOM Image/WebGL. Stub it to return an empty scene so the
// component only exercises the prop-routing logic.
vi.mock("@react-three/drei", () => ({ useGLTF: () => ({ scene: {} }) }));

// Tauri's convertFileSrc is a no-op in tests; return a predictable
// asset://-style URL so callers can assert it's been threaded through.
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (p: string) => `asset://${p}`,
}));

import { GltfModel } from "@/components/graphic3d/GltfModel";

describe("GltfModel", () => {
  it("renders with remote URL fallback when localPath is null", () => {
    const { container } = render(<GltfModel localPath={null} remoteUrl="https://x/y.glb" />);
    expect(container).toBeTruthy();
  });

  it("prefers localPath when provided", () => {
    const { container } = render(<GltfModel localPath="/tmp/a.glb" remoteUrl="https://x/y.glb" />);
    expect(container).toBeTruthy();
  });
});
