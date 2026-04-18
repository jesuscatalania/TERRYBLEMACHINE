import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { StoryboardEditor } from "@/components/video/StoryboardEditor";
import type { Storyboard } from "@/lib/storyboardCommands";

function sampleBoard(): Storyboard {
  return {
    summary: "s",
    template: "commercial",
    shots: [
      {
        id: "shot-a",
        index: 1,
        description: "a",
        style: "",
        duration_s: 3,
        camera: "static",
        transition: "cut",
      },
      {
        id: "shot-b",
        index: 2,
        description: "b",
        style: "",
        duration_s: 4,
        camera: "dolly",
        transition: "fade",
      },
    ],
  };
}

describe("StoryboardEditor", () => {
  it("renders empty state when no storyboard", () => {
    render(<StoryboardEditor storyboard={null} onChange={() => {}} />);
    expect(screen.getByText(/No storyboard yet/i)).toBeInTheDocument();
  });

  it("renders each shot", () => {
    render(<StoryboardEditor storyboard={sampleBoard()} onChange={() => {}} />);
    expect(screen.getByTestId("shot-card-shot-a")).toBeInTheDocument();
    expect(screen.getByTestId("shot-card-shot-b")).toBeInTheDocument();
  });

  it("removes a shot and renumbers", () => {
    const onChange = vi.fn();
    render(<StoryboardEditor storyboard={sampleBoard()} onChange={onChange} />);
    fireEvent.click(screen.getByLabelText(/Remove shot 1/));
    expect(onChange).toHaveBeenCalled();
    const next = onChange.mock.calls[0][0];
    expect(next.shots).toHaveLength(1);
    expect(next.shots[0].index).toBe(1);
  });

  it("adds a shot", () => {
    const onChange = vi.fn();
    render(<StoryboardEditor storyboard={sampleBoard()} onChange={onChange} />);
    fireEvent.click(screen.getByRole("button", { name: /add shot/i }));
    const next = onChange.mock.calls[0][0];
    expect(next.shots).toHaveLength(3);
    expect(next.shots[2].index).toBe(3);
    // New shots must carry a stable client-side id so React doesn't remount
    // them when a later reorder renumbers everyone's `index`.
    expect(typeof next.shots[2].id).toBe("string");
    expect(next.shots[2].id.length).toBeGreaterThan(0);
  });

  it("preserves shot id on reorder so React does not remount inputs", () => {
    const onChange = vi.fn();
    render(<StoryboardEditor storyboard={sampleBoard()} onChange={onChange} />);
    // Simulate drag-drop: start at shot 1, drop on shot 2.
    const first = screen.getByTestId("shot-card-shot-a");
    const second = screen.getByTestId("shot-card-shot-b");
    fireEvent.dragStart(first);
    fireEvent.dragOver(second);
    fireEvent.drop(second);
    const next = onChange.mock.calls[0][0];
    expect(next.shots[0].id).toBe("shot-b");
    expect(next.shots[1].id).toBe("shot-a");
    // Indexes get renumbered to 1..N after reorder.
    expect(next.shots[0].index).toBe(1);
    expect(next.shots[1].index).toBe(2);
  });

  it("updates shot description", () => {
    const onChange = vi.fn();
    render(<StoryboardEditor storyboard={sampleBoard()} onChange={onChange} />);
    const firstDesc = screen.getByLabelText(/Shot 1 description/);
    fireEvent.change(firstDesc, { target: { value: "new desc" } });
    const next = onChange.mock.calls[0][0];
    expect(next.shots[0].description).toBe("new desc");
  });
});
