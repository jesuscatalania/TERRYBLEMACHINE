import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Kbd } from "@/components/shell/Kbd";

describe("Kbd", () => {
  it("renders the shortcut text", () => {
    render(<Kbd>⌘K</Kbd>);
    expect(screen.getByText("⌘K")).toBeInTheDocument();
  });

  it("uses <kbd> element for semantics", () => {
    const { container } = render(<Kbd>⌘1</Kbd>);
    expect(container.querySelector("kbd")).not.toBeNull();
  });
});
