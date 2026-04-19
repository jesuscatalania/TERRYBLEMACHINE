import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { OptimizeToggle } from "@/components/ui/OptimizeToggle";

describe("OptimizeToggle", () => {
  it("shows OFF state when not enabled", () => {
    render(
      <OptimizeToggle
        enabled={false}
        onToggle={() => {}}
        busy={false}
        canUndo={false}
        onUndo={() => {}}
      />,
    );
    expect(screen.getByRole("switch", { name: /Optimize/i })).toHaveAttribute(
      "aria-checked",
      "false",
    );
  });

  it("toggles on click", async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(
      <OptimizeToggle
        enabled={false}
        onToggle={onToggle}
        busy={false}
        canUndo={false}
        onUndo={() => {}}
      />,
    );
    await user.click(screen.getByRole("switch"));
    expect(onToggle).toHaveBeenCalledWith(true);
  });

  it("shows Undo button when canUndo", () => {
    render(
      <OptimizeToggle
        enabled={true}
        onToggle={() => {}}
        busy={false}
        canUndo={true}
        onUndo={() => {}}
      />,
    );
    expect(screen.getByRole("button", { name: /Undo/i })).toBeInTheDocument();
  });

  it("Undo button calls onUndo", async () => {
    const user = userEvent.setup();
    const onUndo = vi.fn();
    render(
      <OptimizeToggle
        enabled={true}
        onToggle={() => {}}
        busy={false}
        canUndo={true}
        onUndo={onUndo}
      />,
    );
    await user.click(screen.getByRole("button", { name: /Undo/i }));
    expect(onUndo).toHaveBeenCalledTimes(1);
  });

  it("shows busy spinner when busy", () => {
    render(
      <OptimizeToggle
        enabled={true}
        onToggle={() => {}}
        busy={true}
        canUndo={false}
        onUndo={() => {}}
      />,
    );
    expect(screen.getByRole("switch")).toHaveAttribute("aria-busy", "true");
  });

  it("disables switch when busy", () => {
    render(
      <OptimizeToggle
        enabled={true}
        onToggle={() => {}}
        busy={true}
        canUndo={false}
        onUndo={() => {}}
      />,
    );
    expect(screen.getByRole("switch")).toBeDisabled();
  });
});
