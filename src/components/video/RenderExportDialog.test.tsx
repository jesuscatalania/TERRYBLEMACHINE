import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { RenderExportDialog } from "@/components/video/RenderExportDialog";

describe("RenderExportDialog", () => {
  it("renders nothing when closed", () => {
    render(<RenderExportDialog open={false} onClose={() => {}} onExport={() => {}} />);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("lists resolution, format, fps options when open", () => {
    render(<RenderExportDialog open onClose={() => {}} onExport={() => {}} />);
    expect(screen.getByLabelText(/resolution/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/format/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/fps/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/filename/i)).toBeInTheDocument();
  });

  it("forwards defaults on export", () => {
    const onExport = vi.fn();
    render(<RenderExportDialog open onClose={() => {}} onExport={onExport} />);
    fireEvent.click(screen.getByRole("button", { name: /^export$/i }));
    expect(onExport).toHaveBeenCalledWith({
      resolution: "hd",
      format: "mp4",
      fps: 30,
      filename: "terryble-video",
    });
  });

  it("forwards changed values", () => {
    const onExport = vi.fn();
    render(<RenderExportDialog open onClose={() => {}} onExport={onExport} />);
    fireEvent.change(screen.getByLabelText(/resolution/i), { target: { value: "1080" } });
    fireEvent.change(screen.getByLabelText(/format/i), { target: { value: "gif" } });
    fireEvent.change(screen.getByLabelText(/fps/i), { target: { value: "60" } });
    fireEvent.change(screen.getByLabelText(/filename/i), { target: { value: "custom-name" } });
    fireEvent.click(screen.getByRole("button", { name: /^export$/i }));
    expect(onExport).toHaveBeenCalledWith({
      resolution: "1080",
      format: "gif",
      fps: 60,
      filename: "custom-name",
    });
  });
});
