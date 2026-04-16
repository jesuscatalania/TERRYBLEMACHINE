import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StatusDot } from "@/components/shell/StatusDot";

describe("StatusDot", () => {
  it("renders with ok status by default", () => {
    const { container } = render(<StatusDot />);
    const dot = container.querySelector('[data-status="ok"]');
    expect(dot).not.toBeNull();
  });

  it("applies warn status when prop is warn", () => {
    const { container } = render(<StatusDot status="warn" />);
    expect(container.querySelector('[data-status="warn"]')).not.toBeNull();
  });

  it("applies error status when prop is error", () => {
    const { container } = render(<StatusDot status="error" />);
    expect(container.querySelector('[data-status="error"]')).not.toBeNull();
  });

  it("exposes an accessible label when given", () => {
    const { getByLabelText } = render(<StatusDot status="ok" label="AI idle" />);
    expect(getByLabelText("AI idle")).toBeInTheDocument();
  });
});
