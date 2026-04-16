import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ModulePlaceholder } from "@/pages/ModulePlaceholder";

describe("ModulePlaceholder", () => {
  it("renders the module label in the coming-soon heading", () => {
    render(<ModulePlaceholder moduleId="video" />);
    expect(screen.getByRole("heading", { name: /coming soon/i })).toBeInTheDocument();
    expect(screen.getByText(/video/i)).toBeInTheDocument();
  });

  it("shows the module's mod tag", () => {
    render(<ModulePlaceholder moduleId="graphic3d" />);
    expect(screen.getAllByText(/MOD—03/).length).toBeGreaterThan(0);
  });
});
