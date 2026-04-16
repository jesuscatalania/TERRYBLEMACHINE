import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SchematicFrame } from "@/components/shell/SchematicFrame";

describe("SchematicFrame", () => {
  it("renders children inside the frame", () => {
    render(
      <SchematicFrame>
        <p>inside</p>
      </SchematicFrame>,
    );
    expect(screen.getByText("inside")).toBeInTheDocument();
  });

  it("renders a figure label when provided", () => {
    render(
      <SchematicFrame figLabel="FIG 01 — READY">
        <span>x</span>
      </SchematicFrame>,
    );
    expect(screen.getByText("FIG 01 — READY")).toBeInTheDocument();
  });

  it("renders a tag (module code) when provided", () => {
    render(
      <SchematicFrame tag="MOD—01">
        <span>x</span>
      </SchematicFrame>,
    );
    expect(screen.getByText("MOD—01")).toBeInTheDocument();
  });

  it("renders 4 corner brackets", () => {
    const { container } = render(
      <SchematicFrame>
        <span>x</span>
      </SchematicFrame>,
    );
    expect(container.querySelectorAll("[data-bracket]")).toHaveLength(4);
  });
});
