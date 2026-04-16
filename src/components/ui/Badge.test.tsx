import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Badge } from "@/components/ui/Badge";

describe("Badge", () => {
  it("renders its label", () => {
    render(<Badge>NEW</Badge>);
    expect(screen.getByText("NEW")).toBeInTheDocument();
  });

  it.each([
    "neutral",
    "success",
    "warn",
    "error",
    "accent",
  ] as const)("marks tone %s via data-tone", (tone) => {
    const { container } = render(<Badge tone={tone}>x</Badge>);
    expect(container.querySelector(`[data-tone="${tone}"]`)).not.toBeNull();
  });

  it("defaults to neutral tone", () => {
    const { container } = render(<Badge>x</Badge>);
    expect(container.querySelector('[data-tone="neutral"]')).not.toBeNull();
  });
});
