import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Breadcrumbs } from "@/components/shell/Breadcrumbs";

describe("Breadcrumbs", () => {
  it("renders parts with separators", () => {
    render(<Breadcrumbs parts={["TM", "WEBSITE", "UNTITLED"]} />);
    expect(screen.getByText("TM")).toBeInTheDocument();
    expect(screen.getByText("WEBSITE")).toBeInTheDocument();
    expect(screen.getByText("UNTITLED")).toBeInTheDocument();
    const seps = screen.getAllByText("/");
    expect(seps.length).toBe(2);
  });

  it("marks the last part as aria-current=page", () => {
    render(<Breadcrumbs parts={["A", "B"]} />);
    expect(screen.getByText("B")).toHaveAttribute("aria-current", "page");
  });
});
