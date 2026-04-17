import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { TextControls } from "@/components/graphic2d/TextControls";

describe("TextControls", () => {
  it("invokes onChange when font changes (after font load resolves)", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<TextControls onChange={onChange} />);
    // Dropdown is a custom combobox button — click it, then click an option.
    await user.click(screen.getByRole("combobox"));
    await user.click(screen.getByRole("option", { name: "Roboto" }));
    // Because the font handler now awaits injectGoogleFont → document.fonts.load,
    // assert via waitFor rather than directly after the click.
    await waitFor(() => expect(onChange).toHaveBeenCalledWith({ font: "Roboto" }));
  });

  it("invokes onChange when color changes", () => {
    const onChange = vi.fn();
    render(<TextControls onChange={onChange} />);
    const color = screen.getByLabelText(/color/i) as HTMLInputElement;
    fireEvent.change(color, { target: { value: "#ff0000" } });
    expect(onChange).toHaveBeenCalledWith({ color: "#ff0000" });
  });

  it("invokes onChange when size changes", () => {
    const onChange = vi.fn();
    render(<TextControls onChange={onChange} />);
    const size = screen.getByLabelText(/size/i) as HTMLInputElement;
    fireEvent.change(size, { target: { value: "72" } });
    expect(onChange).toHaveBeenCalledWith({ size: 72 });
  });

  it("ignores non-positive size values", () => {
    const onChange = vi.fn();
    render(<TextControls onChange={onChange} />);
    const size = screen.getByLabelText(/size/i) as HTMLInputElement;
    fireEvent.change(size, { target: { value: "0" } });
    expect(onChange).not.toHaveBeenCalled();
  });

  it("renders initialFont as the current font-picker value", () => {
    render(<TextControls initialFont="Oswald" onChange={vi.fn()} />);
    // Dropdown's combobox shows the selected label.
    expect(screen.getByRole("combobox")).toHaveTextContent("Oswald");
  });

  it("falls back to Inter when initialFont is not in GOOGLE_FONTS", () => {
    render(<TextControls initialFont="Comic Sans MS" onChange={vi.fn()} />);
    expect(screen.getByRole("combobox")).toHaveTextContent("Inter");
  });

  it("renders initial color and size", () => {
    render(<TextControls initialColor="#abcdef" initialSize={96} onChange={vi.fn()} />);
    const color = screen.getByLabelText(/color/i) as HTMLInputElement;
    expect(color.value).toBe("#abcdef");
    const size = screen.getByLabelText(/size/i) as HTMLInputElement;
    expect(size.value).toBe("96");
  });
});
