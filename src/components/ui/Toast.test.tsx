import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Toaster } from "@/components/ui/Toast";
import { useUiStore } from "@/stores/uiStore";

describe("Toaster", () => {
  beforeEach(() => {
    useUiStore.setState({ modals: [], notifications: [], loadingJobs: 0 });
  });

  afterEach(() => {
    // Ensure no vi.useFakeTimers() state bleeds between tests.
    vi.useRealTimers();
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

  it("renders progress bar when progress is present", () => {
    useUiStore.getState().notify({
      kind: "info",
      message: "Rendering",
      progress: { current: 3, total: 10 },
    });
    render(<Toaster />);
    const bar = screen.getByTestId("toast-progress");
    expect(bar).toBeInTheDocument();
    expect(bar).toHaveStyle({ width: "30%" });
  });

  it("does NOT render a progress bar when progress is absent", () => {
    useUiStore.getState().notify({ kind: "info", message: "Hello" });
    render(<Toaster />);
    expect(screen.queryByTestId("toast-progress")).not.toBeInTheDocument();
  });

  // Regression for debug-review I4: pushing a new toast must NOT reset the
  // dismiss-timer of toasts that are already visible. Toast A pushed at t=0
  // with autoDismissMs=5000 has to go away at t=5000 regardless of B's
  // arrival at t=3000.
  it("uses per-id dismiss timers and does not reset older toasts on new pushes", () => {
    vi.useFakeTimers();
    useUiStore.getState().notify({ kind: "info", message: "A" });
    render(<Toaster autoDismissMs={5000} />);
    expect(useUiStore.getState().notifications).toHaveLength(1);

    act(() => {
      vi.advanceTimersByTime(3000);
    });
    // After 3s, push B. Under the buggy (reset) behavior the effect would
    // rebuild the timer set and A would live another 5s.
    act(() => {
      useUiStore.getState().notify({ kind: "info", message: "B" });
    });
    expect(useUiStore.getState().notifications).toHaveLength(2);

    // Advance another 2500ms (total 5500ms from A, 2500ms from B).
    act(() => {
      vi.advanceTimersByTime(2500);
    });
    const msgs = useUiStore.getState().notifications.map((n) => n.message);
    expect(msgs).not.toContain("A");
    expect(msgs).toContain("B");
  });
});
