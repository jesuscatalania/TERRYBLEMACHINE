import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ThreeExportDialog } from "@/components/graphic3d/ThreeExportDialog";

describe("ThreeExportDialog", () => {
  it("renders nothing when closed", () => {
    render(<ThreeExportDialog open={false} onClose={() => {}} onExport={() => {}} />);
    // Modal returns null when open=false; no dialog in the DOM.
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("lists all 4 formats (PNG/JPEG/WebP/PDF)", async () => {
    const user = userEvent.setup();
    render(<ThreeExportDialog open onClose={() => {}} onExport={() => {}} />);
    await user.click(screen.getByRole("combobox"));
    expect(screen.getByRole("option", { name: /PNG/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /JPEG/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /WebP/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /PDF/i })).toBeInTheDocument();
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
});
