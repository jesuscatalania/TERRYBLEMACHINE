import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { WebsiteExportDialog } from "@/components/website/WebsiteExportDialog";

describe("WebsiteExportDialog", () => {
  it("does not render when open=false", () => {
    render(<WebsiteExportDialog open={false} onClose={() => {}} onExport={vi.fn()} />);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("renders format and deploy dropdowns when open", () => {
    render(<WebsiteExportDialog open={true} onClose={() => {}} onExport={vi.fn()} />);
    expect(screen.getByLabelText(/format/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/deploy target/i)).toBeInTheDocument();
  });

  it("calls onExport with default settings (raw, no deploy)", async () => {
    const user = userEvent.setup();
    const onExport = vi.fn();
    render(<WebsiteExportDialog open={true} onClose={() => {}} onExport={onExport} />);

    await user.click(screen.getByRole("button", { name: /^export$/i }));

    expect(onExport).toHaveBeenCalledWith({ format: "raw", deploy: undefined });
  });

  it("calls onExport with the selected format and deploy target", async () => {
    const user = userEvent.setup();
    const onExport = vi.fn();
    render(<WebsiteExportDialog open={true} onClose={() => {}} onExport={onExport} />);

    // Open format dropdown and pick React + Vite.
    await user.click(screen.getByLabelText(/format/i));
    await user.click(screen.getByRole("option", { name: /react \+ vite/i }));

    // Open deploy dropdown and pick Vercel.
    await user.click(screen.getByLabelText(/deploy target/i));
    await user.click(screen.getByRole("option", { name: /vercel/i }));

    await user.click(screen.getByRole("button", { name: /^export$/i }));

    expect(onExport).toHaveBeenCalledWith({ format: "react", deploy: "vercel" });
  });

  it("invokes onClose when Cancel is clicked", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<WebsiteExportDialog open={true} onClose={onClose} onExport={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: /cancel/i }));
    expect(onClose).toHaveBeenCalled();
  });
});
