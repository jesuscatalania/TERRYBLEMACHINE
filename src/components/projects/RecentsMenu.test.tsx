import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { RecentsMenu } from "@/components/projects/RecentsMenu";
import type { Project } from "@/stores/projectStore";
import { useProjectStore } from "@/stores/projectStore";

const alpha: Project = {
  id: "a",
  name: "Alpha",
  module: "website",
  path: "/tmp/alpha",
  createdAt: "2026-01-01T00:00:00.000Z",
  description: "",
};

const beta: Project = {
  id: "b",
  name: "Beta",
  module: "graphic2d",
  path: "/tmp/beta",
  createdAt: "2026-01-02T00:00:00.000Z",
};

const gamma: Project = {
  id: "c",
  name: "Gamma",
  module: "video",
  path: "/tmp/gamma",
  createdAt: "2026-01-03T00:00:00.000Z",
};

describe("RecentsMenu", () => {
  beforeEach(() => {
    useProjectStore.setState({
      recents: [alpha, beta],
      currentProject: null,
    });
  });

  it("renders each recent entry when opened", () => {
    render(<RecentsMenu />);
    fireEvent.click(screen.getByRole("button", { name: /recent/i }));
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    expect(screen.getByText("Beta")).toBeInTheDocument();
  });

  it("does not render the list until toggled open", () => {
    render(<RecentsMenu />);
    expect(screen.queryByText("Alpha")).not.toBeInTheDocument();
  });

  it("shows empty state when recents is empty", () => {
    useProjectStore.setState({ recents: [] });
    render(<RecentsMenu />);
    fireEvent.click(screen.getByRole("button", { name: /recent/i }));
    expect(screen.getByText(/no recent projects/i)).toBeInTheDocument();
  });

  it("calls openProject and closes menu on click", () => {
    const openProject = vi.fn();
    useProjectStore.setState({ openProject });

    render(<RecentsMenu />);
    fireEvent.click(screen.getByRole("button", { name: /recent/i }));
    fireEvent.click(screen.getByText("Alpha"));

    expect(openProject).toHaveBeenCalledTimes(1);
    expect(openProject).toHaveBeenCalledWith(alpha);
    // Menu should have closed — Alpha is no longer rendered.
    expect(screen.queryByText("Alpha")).not.toBeInTheDocument();
  });

  it("toggles the menu closed on second trigger click", () => {
    render(<RecentsMenu />);
    const trigger = screen.getByRole("button", { name: /recent/i });
    fireEvent.click(trigger);
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    fireEvent.click(trigger);
    expect(screen.queryByText("Alpha")).not.toBeInTheDocument();
  });

  it("closes when clicking outside the menu", () => {
    render(
      <div>
        <div data-testid="outside">outside</div>
        <RecentsMenu />
      </div>,
    );
    fireEvent.click(screen.getByRole("button", { name: /recent/i }));
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    fireEvent.mouseDown(screen.getByTestId("outside"));
    expect(screen.queryByText("Alpha")).not.toBeInTheDocument();
  });

  it("closes on Escape and returns focus to the trigger", async () => {
    const user = userEvent.setup();
    render(<RecentsMenu />);
    const trigger = screen.getByRole("button", { name: /recent/i });
    await user.click(trigger);
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(screen.queryByText("Alpha")).not.toBeInTheDocument();
    // Focus should have returned to the trigger.
    await vi.waitFor(() => expect(document.activeElement).toBe(trigger));
  });

  it("returns focus to the trigger after item-click selection", async () => {
    const user = userEvent.setup();
    render(<RecentsMenu />);
    const trigger = screen.getByRole("button", { name: /recent/i });
    await user.click(trigger);
    await user.click(screen.getByText("Alpha"));
    await vi.waitFor(() => expect(document.activeElement).toBe(trigger));
  });

  it("ArrowDown / ArrowUp / Home / End move focus across items (roving tabindex)", async () => {
    useProjectStore.setState({ recents: [alpha, beta, gamma] });
    const user = userEvent.setup();
    render(<RecentsMenu />);
    await user.click(screen.getByRole("button", { name: /recent/i }));

    // First item auto-focuses once the menu opens.
    const items = screen.getAllByRole("menuitem");
    await vi.waitFor(() => expect(document.activeElement).toBe(items[0]));

    await user.keyboard("{ArrowDown}");
    await vi.waitFor(() => expect(document.activeElement).toBe(items[1]));
    await user.keyboard("{End}");
    await vi.waitFor(() => expect(document.activeElement).toBe(items[2]));
    // Wraps from last to first.
    await user.keyboard("{ArrowDown}");
    await vi.waitFor(() => expect(document.activeElement).toBe(items[0]));
    // ArrowUp from first wraps to last.
    await user.keyboard("{ArrowUp}");
    await vi.waitFor(() => expect(document.activeElement).toBe(items[2]));
    await user.keyboard("{Home}");
    await vi.waitFor(() => expect(document.activeElement).toBe(items[0]));
  });
});
