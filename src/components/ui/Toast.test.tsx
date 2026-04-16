import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { Toaster } from "@/components/ui/Toast";
import { useUiStore } from "@/stores/uiStore";

describe("Toaster", () => {
  beforeEach(() => {
    useUiStore.setState({ modals: [], notifications: [], loadingJobs: 0 });
  });

  it("renders no toasts by default", () => {
    render(<Toaster />);
    expect(screen.queryAllByRole("status")).toHaveLength(0);
  });

  it("renders a toast when notify() is called", () => {
    useUiStore.getState().notify({ kind: "success", message: "Saved!" });
    render(<Toaster />);
    const toast = screen.getByRole("status");
    expect(toast).toHaveTextContent("Saved!");
    expect(toast).toHaveAttribute("data-kind", "success");
  });

  it("renders multiple toasts in order", () => {
    useUiStore.getState().notify({ kind: "info", message: "One" });
    useUiStore.getState().notify({ kind: "error", message: "Two" });
    render(<Toaster />);
    const toasts = screen.getAllByRole("status");
    expect(toasts).toHaveLength(2);
    expect(toasts[0]).toHaveTextContent("One");
    expect(toasts[1]).toHaveTextContent("Two");
  });

  it("dismisses a toast when its close button is clicked", async () => {
    const user = userEvent.setup();
    useUiStore.getState().notify({ kind: "info", message: "Dismiss me" });
    render(<Toaster />);
    await user.click(screen.getByRole("button", { name: /dismiss/i }));
    expect(useUiStore.getState().notifications).toHaveLength(0);
  });
});
