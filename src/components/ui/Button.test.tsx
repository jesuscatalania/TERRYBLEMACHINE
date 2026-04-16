import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Button } from "@/components/ui/Button";

describe("Button", () => {
  it("renders children", () => {
    render(<Button>Generate</Button>);
    expect(screen.getByRole("button", { name: "Generate" })).toBeInTheDocument();
  });

  it("defaults to secondary + md", () => {
    const { container } = render(<Button>Go</Button>);
    const btn = container.querySelector("button");
    expect(btn).toHaveAttribute("data-variant", "secondary");
    expect(btn).toHaveAttribute("data-size", "md");
  });

  it.each([
    "primary",
    "secondary",
    "ghost",
    "danger",
    "icon",
  ] as const)("marks variant %s", (variant) => {
    const { container } = render(<Button variant={variant}>x</Button>);
    expect(container.querySelector(`button[data-variant="${variant}"]`)).not.toBeNull();
  });

  it.each(["sm", "md", "lg"] as const)("marks size %s", (size) => {
    const { container } = render(<Button size={size}>x</Button>);
    expect(container.querySelector(`button[data-size="${size}"]`)).not.toBeNull();
  });

  it("fires onClick", async () => {
    const user = userEvent.setup();
    const handler = vi.fn();
    render(<Button onClick={handler}>Go</Button>);
    await user.click(screen.getByRole("button"));
    expect(handler).toHaveBeenCalledOnce();
  });

  it("respects disabled", () => {
    render(<Button disabled>Done</Button>);
    expect(screen.getByRole("button")).toBeDisabled();
  });
});
