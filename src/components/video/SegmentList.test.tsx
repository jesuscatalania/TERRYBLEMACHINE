import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SegmentList } from "@/components/video/SegmentList";
import type { Segment } from "@/stores/videoStore";

function sample(): Segment[] {
  return [
    { id: "a", kind: "ai", label: "Shot 1", duration_s: 5 },
    { id: "b", kind: "remotion", label: "Kinetic", duration_s: 3 },
  ];
}

describe("SegmentList", () => {
  it("renders empty state when no segments", () => {
    render(<SegmentList segments={[]} onDelete={() => {}} onReorder={() => {}} />);
    expect(screen.getByText(/No segments yet/i)).toBeInTheDocument();
  });

  it("renders each segment", () => {
    render(<SegmentList segments={sample()} onDelete={() => {}} onReorder={() => {}} />);
    expect(screen.getByTestId("segment-a")).toBeInTheDocument();
    expect(screen.getByTestId("segment-b")).toBeInTheDocument();
    expect(screen.getByText("Shot 1")).toBeInTheDocument();
    expect(screen.getByText("Kinetic")).toBeInTheDocument();
  });

  it("delete calls onDelete with segment id", () => {
    const onDelete = vi.fn();
    render(<SegmentList segments={sample()} onDelete={onDelete} onReorder={() => {}} />);
    fireEvent.click(screen.getByLabelText(/Delete segment Shot 1/));
    expect(onDelete).toHaveBeenCalledWith("a");
  });

  it("select calls onSelect when segment clicked", () => {
    const onSelect = vi.fn();
    render(
      <SegmentList
        segments={sample()}
        onDelete={() => {}}
        onReorder={() => {}}
        onSelect={onSelect}
      />,
    );
    fireEvent.click(screen.getByText("Shot 1"));
    expect(onSelect).toHaveBeenCalledWith("a");
  });

  it("shows busy indicator", () => {
    const segs: Segment[] = [{ id: "a", kind: "ai", label: "X", duration_s: 5, busy: true }];
    render(<SegmentList segments={segs} onDelete={() => {}} onReorder={() => {}} />);
    expect(screen.getByTestId("segment-a").textContent).toContain("…");
  });

  it("shows error indicator", () => {
    const segs: Segment[] = [{ id: "a", kind: "ai", label: "X", duration_s: 5, error: "boom" }];
    render(<SegmentList segments={segs} onDelete={() => {}} onReorder={() => {}} />);
    expect(screen.getByTestId("segment-a").textContent).toContain("!");
  });
});
