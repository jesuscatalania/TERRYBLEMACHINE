import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { AssistPopover } from "@/components/website/AssistPopover";

describe("AssistPopover", () => {
  it("renders the provided selection as read-only context", () => {
    render(
      <AssistPopover
        selection="<h1>Headline</h1>"
        onSubmit={() => {}}
        onClose={() => {}}
        busy={false}
      />,
    );
    expect(screen.getByTestId("assist-selection")).toHaveTextContent("<h1>Headline</h1>");
  });

  it("truncates very long selections to 500 chars", () => {
    const long = "x".repeat(800);
    render(<AssistPopover selection={long} onSubmit={() => {}} onClose={() => {}} busy={false} />);
    expect(screen.getByTestId("assist-selection").textContent ?? "").toHaveLength(500);
  });

  it("disables Apply while instruction is empty", () => {
    render(<AssistPopover selection="x" onSubmit={() => {}} onClose={() => {}} busy={false} />);
    expect(screen.getByRole("button", { name: /apply/i })).toBeDisabled();
  });

  it("enables Apply once instruction is non-empty", async () => {
    const user = userEvent.setup();
    render(<AssistPopover selection="x" onSubmit={() => {}} onClose={() => {}} busy={false} />);
    await user.type(screen.getByLabelText(/change to/i), "make it red");
    expect(screen.getByRole("button", { name: /apply/i })).toBeEnabled();
  });

  it("calls onSubmit with the trimmed instruction when Apply is clicked", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(<AssistPopover selection="x" onSubmit={onSubmit} onClose={() => {}} busy={false} />);
    await user.type(screen.getByLabelText(/change to/i), "   make it bold   ");
    await user.click(screen.getByRole("button", { name: /apply/i }));
    expect(onSubmit).toHaveBeenCalledWith("make it bold");
  });

  it("calls onClose when Cancel is clicked", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<AssistPopover selection="x" onSubmit={() => {}} onClose={onClose} busy={false} />);
    await user.click(screen.getByRole("button", { name: /cancel/i }));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("shows loading spinner on Apply and disables both buttons while busy", () => {
    render(<AssistPopover selection="x" onSubmit={() => {}} onClose={() => {}} busy={true} />);
    expect(screen.getByRole("button", { name: /apply/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /cancel/i })).toBeDisabled();
    expect(screen.getByTestId("loading-spinner")).toBeInTheDocument();
  });
});
