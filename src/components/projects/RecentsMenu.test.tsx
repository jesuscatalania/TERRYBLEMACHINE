import { fireEvent, render, screen } from "@testing-library/react";
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
});
