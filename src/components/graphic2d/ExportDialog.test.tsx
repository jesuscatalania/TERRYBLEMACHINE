import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/graphic2d/ExportDialog";

describe("ExportDialog", () => {
  it("shows PDF and GIF in format options", async () => {
    const user = userEvent.setup();
    render(<ExportDialog open onClose={() => {}} onExport={() => {}} />);
    await user.click(screen.getByRole("combobox"));
    expect(screen.getByRole("option", { name: /PDF/i })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: /GIF/i })).toBeInTheDocument();
  });

  it("passes pdf format through onExport", async () => {
    const user = userEvent.setup();
    const onExport = vi.fn();
    render(<ExportDialog open onClose={() => {}} onExport={onExport} />);
    await user.click(screen.getByRole("combobox"));
    await user.click(screen.getByRole("option", { name: /PDF/i }));
    await user.click(screen.getByRole("button", { name: /^Export$/i }));
    expect(onExport).toHaveBeenCalledWith(expect.objectContaining({ format: "pdf" }));
  });

  it("reveals Frames + Delay inputs when GIF is selected and forwards them", async () => {
    const user = userEvent.setup();
    const onExport = vi.fn();
    render(<ExportDialog open onClose={() => {}} onExport={onExport} />);
    await user.click(screen.getByRole("combobox"));
    await user.click(screen.getByRole("option", { name: /GIF/i }));
    expect(screen.getByLabelText(/Frames/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Delay/i)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /^Export$/i }));
    expect(onExport).toHaveBeenCalledWith(
      expect.objectContaining({ format: "gif", frames: 1, delayMs: 100 }),
    );
  });
});
