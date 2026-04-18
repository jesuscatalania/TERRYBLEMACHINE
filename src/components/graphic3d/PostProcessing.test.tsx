import { render } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";

// Stub @react-three/postprocessing so the component renders in jsdom without
// WebGL. EffectComposer normally requires the R3F context and a live
// WebGLRenderer; here we just need to verify the component's short-circuit
// logic and which effects are rendered for which prop combinations.
vi.mock("@react-three/postprocessing", () => ({
  EffectComposer: (props: { children?: ReactNode }) => (
    <div data-testid="effect-composer">{props.children}</div>
  ),
  Bloom: () => <div data-testid="effect-bloom" />,
  SSAO: () => <div data-testid="effect-ssao" />,
}));

import { PostProcessing } from "@/components/graphic3d/PostProcessing";

describe("PostProcessing", () => {
  it("renders nothing when both flags are off", () => {
    const { container } = render(<PostProcessing bloom={false} ssao={false} />);
    expect(container.firstChild).toBeNull();
  });

  it("renders the composer when bloom is on", () => {
    const { getByTestId, queryByTestId } = render(<PostProcessing bloom ssao={false} />);
    expect(getByTestId("effect-composer")).toBeInTheDocument();
    expect(getByTestId("effect-bloom")).toBeInTheDocument();
    expect(queryByTestId("effect-ssao")).toBeNull();
  });

  it("renders both effects when both flags are on", () => {
    const { getByTestId } = render(<PostProcessing bloom ssao />);
    expect(getByTestId("effect-composer")).toBeInTheDocument();
    expect(getByTestId("effect-bloom")).toBeInTheDocument();
    expect(getByTestId("effect-ssao")).toBeInTheDocument();
  });
});
