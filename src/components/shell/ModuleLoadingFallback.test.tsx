import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ModuleLoadingFallback } from "@/components/shell/ModuleLoadingFallback";

describe("ModuleLoadingFallback", () => {
  it("renders without crashing", () => {
    render(<ModuleLoadingFallback />);
    const wrapper = screen.getByRole("status");
    expect(wrapper).toBeInTheDocument();
    expect(wrapper).toHaveAttribute("aria-busy", "true");
  });

  it("contains skeleton placeholders", () => {
    const { container } = render(<ModuleLoadingFallback />);
    const skeletons = container.querySelectorAll("[data-skeleton='true']");
    expect(skeletons.length).toBeGreaterThanOrEqual(4);
  });
});
