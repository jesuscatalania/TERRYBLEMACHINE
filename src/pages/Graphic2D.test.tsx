import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

// Stub the fabric-backed canvas so the page renders in jsdom without
// pulling in Fabric's full object model. The real canvas is exercised by
// src/components/graphic2d/FabricCanvas.test.tsx.
vi.mock("@/components/graphic2d/FabricCanvas", () => ({
  FabricCanvas: () => <div data-testid="fabric-canvas-stub" />,
}));

vi.mock("@/lib/imageCommands", () => ({
  generateVariants: vi.fn(async () => [
    {
      url: "https://fake.fal/variant-1.png",
      width: 1024,
      height: 1024,
      model: "FalFluxPro",
      cached: false,
    },
  ]),
  inpaintImage: vi.fn(),
  isDataUrl: (s: string) => /^data:/i.test(s),
}));

import { generateVariants } from "@/lib/imageCommands";
import { Graphic2DPage } from "@/pages/Graphic2D";

describe("Graphic2DPage — /tool override + ToolDropdown wiring (T18)", () => {
  beforeEach(() => {
    vi.mocked(generateVariants).mockClear();
  });

  it("parses `/flux cat sunset` prompt: model_override=FalFluxPro, cleanPrompt=cat sunset", async () => {
    render(<Graphic2DPage />);
    fireEvent.change(screen.getByLabelText(/describe the image/i), {
      target: { value: "/flux cat sunset" },
    });
    const generateBtn = screen.getByRole("button", { name: /generate 4 variants/i });
    expect(generateBtn).not.toBeDisabled();
    fireEvent.click(generateBtn);
    await waitFor(() => expect(generateVariants).toHaveBeenCalledTimes(1));
    expect(vi.mocked(generateVariants).mock.calls[0]?.[0]).toMatchObject({
      prompt: "cat sunset",
      count: 4,
      module: "graphic2d",
      model_override: "FalFluxPro",
    });
  });

  it("sends model_override=undefined when no slug + dropdown is Auto", async () => {
    render(<Graphic2DPage />);
    fireEvent.change(screen.getByLabelText(/describe the image/i), {
      target: { value: "just a plain prompt" },
    });
    fireEvent.click(screen.getByRole("button", { name: /generate 4 variants/i }));
    await waitFor(() => expect(generateVariants).toHaveBeenCalledTimes(1));
    const call = vi.mocked(generateVariants).mock.calls[0]?.[0];
    expect(call?.prompt).toBe("just a plain prompt");
    expect(call?.model_override).toBeUndefined();
  });
});
