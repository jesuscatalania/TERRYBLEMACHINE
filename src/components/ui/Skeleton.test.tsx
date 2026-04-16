import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Skeleton } from "@/components/ui/Skeleton";

describe("Skeleton", () => {
  it("renders a pulsing placeholder element", () => {
    const { container } = render(<Skeleton />);
    const el = container.querySelector('[data-skeleton="true"]');
    expect(el).not.toBeNull();
    expect(el?.className).toMatch(/animate-pulse/);
  });

  it("respects width/height numeric props", () => {
    const { container } = render(<Skeleton width={200} height={24} />);
    const el = container.querySelector('[data-skeleton="true"]') as HTMLElement;
    expect(el.style.width).toBe("200px");
    expect(el.style.height).toBe("24px");
  });

  it("accepts string dimensions", () => {
    const { container } = render(<Skeleton width="50%" height="1rem" />);
    const el = container.querySelector('[data-skeleton="true"]') as HTMLElement;
    expect(el.style.width).toBe("50%");
    expect(el.style.height).toBe("1rem");
  });
});
