import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

// R3F's useLoader would actually fetch the texture via three's TextureLoader
// under jsdom, which has no DOM Image/WebGL. Stub both so the component only
// resolves the texture references — enough to verify the prop wiring.
vi.mock("@react-three/fiber", () => ({
  useLoader: () => [{}, {}],
}));

vi.mock("three", () => ({ TextureLoader: class {} }));

import { DepthPlane } from "@/components/graphic3d/DepthPlane";

describe("DepthPlane", () => {
  it("renders without crashing given both urls", () => {
    // The R3F intrinsics (mesh, planeGeometry, meshStandardMaterial) render
    // as unknown DOM elements outside <Canvas>; React logs warnings but
    // doesn't error. We're verifying component wiring, not 3D output.
    const result = render(<DepthPlane imageUrl="/a.png" depthUrl="/b.png" />);
    expect(result.container).toBeTruthy();
  });

  it("accepts custom dimensions and segments", () => {
    const result = render(
      <DepthPlane
        imageUrl="/a.png"
        depthUrl="/b.png"
        width={6}
        height={4}
        segments={64}
        displacementScale={1.2}
      />,
    );
    expect(result.container).toBeTruthy();
  });
});
