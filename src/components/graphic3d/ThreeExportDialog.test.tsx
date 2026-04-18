import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ThreeExportDialog } from "@/components/graphic3d/ThreeExportDialog";

describe("ThreeExportDialog", () => {
  it("renders nothing when closed", () => {
    render(<ThreeExportDialog open={false} onClose={() => {}} onExport={() => {}} />);
    // Modal returns null when open=false; no dialog in the DOM.
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("lists all 5 formats (PNG/JPEG/WebP/PDF/GIF)", async () => {
    const user = userEvent.setup();
    render(<ThreeExportDialog open onClose={() => {}} onExport={() => {}} />);
    await user.click(screen.getByRole("combobox"));
    expect(screen.getByRole("option", { name: /PNG/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /JPEG/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /WebP/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /PDF/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /GIF/i })).toBeInTheDocument();
  });

  it("passes PDF format through onExport", async () => {
    const user = userEvent.setup();
    const onExport = vi.fn();
    render(<ThreeExportDialog open onClose={() => {}} onExport={onExport} />);
    await user.click(screen.getByRole("combobox"));
    await user.click(screen.getByRole("option", { name: /PDF/i }));
    await user.click(screen.getByRole("button", { name: /^Export$/i }));
    expect(onExport).toHaveBeenCalledWith(expect.objectContaining({ format: "pdf" }));
  });

  it("forwards transparent=true when PNG + checkbox checked", async () => {
    const user = userEvent.setup();
    const onExport = vi.fn();
    render(<ThreeExportDialog open onClose={() => {}} onExport={onExport} />);
    // Format defaults to png, so the transparent checkbox is visible.
    await user.click(screen.getByLabelText(/Transparent/i));
    await user.click(screen.getByRole("button", { name: /^Export$/i }));
    expect(onExport).toHaveBeenCalledWith(
      expect.objectContaining({ format: "png", transparent: true }),
    );
  });

  it("forwards GIF format with default frames/delayMs", async () => {
    const user = userEvent.setup();
    const onExport = vi.fn();
    render(<ThreeExportDialog open onClose={() => {}} onExport={onExport} />);
    await user.click(screen.getByRole("combobox"));
    await user.click(screen.getByRole("option", { name: /GIF/i }));
    await user.click(screen.getByRole("button", { name: /^Export$/i }));
    expect(onExport).toHaveBeenCalledWith(
      expect.objectContaining({ format: "gif", frames: 30, delayMs: 100 }),
    );
  });

  it("shows frames/delay inputs only when format is GIF", async () => {
    const user = userEvent.setup();
    render(<ThreeExportDialog open onClose={() => {}} onExport={() => {}} />);
    // Default is PNG — GIF inputs absent.
    expect(screen.queryByLabelText(/^Frames$/i)).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/^Delay/i)).not.toBeInTheDocument();
    // Switch to GIF.
    await user.click(screen.getByRole("combobox"));
    await user.click(screen.getByRole("option", { name: /GIF/i }));
    expect(screen.getByLabelText(/^Frames$/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^Delay/i)).toBeInTheDocument();
  });

  it("clamps GIF frames to >=1 and delayMs to >=10", async () => {
    const user = userEvent.setup();
    const onExport = vi.fn();
    render(<ThreeExportDialog open onClose={() => {}} onExport={onExport} />);
    await user.click(screen.getByRole("combobox"));
    await user.click(screen.getByRole("option", { name: /GIF/i }));
    const framesInput = screen.getByLabelText(/^Frames$/i) as HTMLInputElement;
    const delayInput = screen.getByLabelText(/^Delay/i) as HTMLInputElement;
    // fireEvent.change sets the value synchronously to the given string,
    // which lets us drive a single "0"/"1" into the clamping logic without
    // userEvent.type's concat behavior.
    fireEvent.change(framesInput, { target: { value: "0" } });
    fireEvent.change(delayInput, { target: { value: "1" } });
    await user.click(screen.getByRole("button", { name: /^Export$/i }));
    expect(onExport).toHaveBeenCalledWith(
      expect.objectContaining({ format: "gif", frames: 1, delayMs: 10 }),
    );
  });
});
