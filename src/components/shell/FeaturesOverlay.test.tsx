import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { FeaturesOverlay } from "@/components/shell/FeaturesOverlay";

describe("FeaturesOverlay", () => {
  it("renders nothing when closed", () => {
    render(<FeaturesOverlay open={false} onClose={() => {}} />);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("renders the 'Was kann das?' heading when open", () => {
    render(<FeaturesOverlay open={true} onClose={() => {}} />);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText("Was kann das?")).toBeInTheDocument();
  });

  it("contains the key content anchors (slugs, models, features)", () => {
    render(<FeaturesOverlay open={true} onClose={() => {}} />);
    // Use getAllByText throughout — several anchors appear multiple times
    // (slug pills in both ecosystem cards and the cheatsheet, model names
    // in card titles and in narrative text).
    expect(screen.getAllByText("/flux").length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Kling V2 Master/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Optimize/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Taste-Engine/).length).toBeGreaterThan(0);
  });
});
