import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Button } from "@/components/shell/Button";

describe("Button", () => {
  it("renders its children", () => {
    render(<Button>Generate</Button>);
    expect(screen.getByRole("button", { name: "Generate" })).toBeInTheDocument();
  });

  it("fires onClick", async () => {
    const user = userEvent.setup();
    const handler = vi.fn();
    render(<Button onClick={handler}>Go</Button>);
    await user.click(screen.getByRole("button", { name: "Go" }));
    expect(handler).toHaveBeenCalledOnce();
  });

  it("marks variant via data-variant", () => {
    const { container } = render(<Button variant="primary">Go</Button>);
    expect(container.querySelector('button[data-variant="primary"]')).not.toBeNull();
  });

  it("defaults to secondary variant", () => {
    const { container } = render(<Button>Go</Button>);
    expect(container.querySelector('button[data-variant="secondary"]')).not.toBeNull();
  });

  it("forwards disabled", () => {
    render(<Button disabled>Done</Button>);
    expect(screen.getByRole("button", { name: "Done" })).toBeDisabled();
  });
});
