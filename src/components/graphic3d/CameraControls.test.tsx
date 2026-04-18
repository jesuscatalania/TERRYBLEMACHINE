import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CameraControls } from "@/components/graphic3d/CameraControls";

describe("CameraControls", () => {
  it("defaults to the given mode", () => {
    render(<CameraControls mode="perspective" onModeChange={() => {}} />);
    expect(screen.getByRole("combobox")).toHaveValue("perspective");
  });

  it("calls onModeChange when switched to orthographic", () => {
    const onChange = vi.fn();
    render(<CameraControls mode="perspective" onModeChange={onChange} />);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "orthographic" } });
    expect(onChange).toHaveBeenCalledWith("orthographic");
  });

  it("renders both options", () => {
    render(<CameraControls mode="perspective" onModeChange={() => {}} />);
    expect(screen.getByRole("option", { name: /perspective/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /orthographic/i })).toBeInTheDocument();
  });
});
