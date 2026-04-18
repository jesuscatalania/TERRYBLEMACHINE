import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { TextLogoControls, type TextStyle } from "@/components/typography/TextLogoControls";

const BASE: TextStyle = {
  font: "Inter",
  color: "#F7F7F8",
  size: 72,
  kerning: 0,
  tracking: 0,
};

describe("TextLogoControls", () => {
  it("renders every control with its current value", () => {
    render(<TextLogoControls value={BASE} onChange={vi.fn()} />);
    expect(screen.getByLabelText(/font/i)).toHaveValue("Inter");
    expect((screen.getByLabelText(/color/i) as HTMLInputElement).value).toBe("#f7f7f8");
    expect((screen.getByLabelText(/size/i) as HTMLInputElement).value).toBe("72");
    expect((screen.getByLabelText(/kerning/i) as HTMLInputElement).value).toBe("0");
    expect((screen.getByLabelText(/tracking/i) as HTMLInputElement).value).toBe("0");
  });

  it("fires onChange with font patch (after font load resolves)", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<TextLogoControls value={BASE} onChange={onChange} />);
    await user.selectOptions(screen.getByLabelText(/font/i), "Roboto");
    await waitFor(() => expect(onChange).toHaveBeenCalledWith({ ...BASE, font: "Roboto" }));
  });

  it("fires onChange with color patch", () => {
    const onChange = vi.fn();
    render(<TextLogoControls value={BASE} onChange={onChange} />);
    fireEvent.change(screen.getByLabelText(/color/i), {
      target: { value: "#ff0000" },
    });
    expect(onChange).toHaveBeenCalledWith({ ...BASE, color: "#ff0000" });
  });

  it("fires onChange with size patch", () => {
    const onChange = vi.fn();
    render(<TextLogoControls value={BASE} onChange={onChange} />);
    fireEvent.change(screen.getByLabelText(/size/i), {
      target: { value: "120" },
    });
    expect(onChange).toHaveBeenCalledWith({ ...BASE, size: 120 });
  });

  it("fires onChange with kerning patch", () => {
    const onChange = vi.fn();
    render(<TextLogoControls value={BASE} onChange={onChange} />);
    fireEvent.change(screen.getByLabelText(/kerning/i), {
      target: { value: "5.5" },
    });
    expect(onChange).toHaveBeenCalledWith({ ...BASE, kerning: 5.5 });
  });

  it("fires onChange with tracking patch", () => {
    const onChange = vi.fn();
    render(<TextLogoControls value={BASE} onChange={onChange} />);
    fireEvent.change(screen.getByLabelText(/tracking/i), {
      target: { value: "12" },
    });
    expect(onChange).toHaveBeenCalledWith({ ...BASE, tracking: 12 });
  });
});
