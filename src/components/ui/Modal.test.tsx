import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Modal } from "@/components/ui/Modal";
import { useModalStackStore } from "@/stores/modalStackStore";

afterEach(() => {
  useModalStackStore.setState({ stack: [] });
});

describe("Modal", () => {
  it("does not render when open=false", () => {
    render(
      <Modal open={false} onClose={() => {}} title="Hello">
        <p>body</p>
      </Modal>,
    );
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("renders when open=true with title + content", () => {
    render(
      <Modal open={true} onClose={() => {}} title="Hello">
        <p>body</p>
      </Modal>,
    );
    const dialog = screen.getByRole("dialog");
    expect(dialog).toBeInTheDocument();
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(screen.getByText("Hello")).toBeInTheDocument();
    expect(screen.getByText("body")).toBeInTheDocument();
  });

  it("fires onClose when the backdrop is clicked", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(
      <Modal open={true} onClose={onClose} title="Hello">
        <p>body</p>
      </Modal>,
    );
    await user.click(screen.getByTestId("modal-backdrop"));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("fires onClose when the close button is clicked", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(
      <Modal open={true} onClose={onClose} title="Hello">
        <p>body</p>
      </Modal>,
    );
    await user.click(screen.getByRole("button", { name: /close/i }));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("fires onClose on Escape key", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(
      <Modal open={true} onClose={onClose} title="Hello">
        <p>body</p>
      </Modal>,
    );
    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("only the top-of-stack modal closes on Escape when two are open", async () => {
    const user = userEvent.setup();
    const onCloseA = vi.fn();
    const onCloseB = vi.fn();
    render(
      <>
        <Modal open={true} onClose={onCloseA} title="A">
          <p>a</p>
        </Modal>
        <Modal open={true} onClose={onCloseB} title="B">
          <p>b</p>
        </Modal>
      </>,
    );
    // B mounts second → sits on top of the stack. Only B should close.
    await user.keyboard("{Escape}");
    expect(onCloseB).toHaveBeenCalledOnce();
    expect(onCloseA).not.toHaveBeenCalled();
  });

  it("pushes the modal onto the global stack while open", () => {
    const { unmount } = render(
      <Modal open={true} onClose={() => {}} title="Hello">
        <p>body</p>
      </Modal>,
    );
    expect(useModalStackStore.getState().isAnyOpen()).toBe(true);
    unmount();
    expect(useModalStackStore.getState().isAnyOpen()).toBe(false);
  });
});
