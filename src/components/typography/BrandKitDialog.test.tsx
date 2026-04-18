import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { BrandKitDialog } from "@/components/typography/BrandKitDialog";

describe("BrandKitDialog", () => {
  it("does not render when open=false", () => {
    render(<BrandKitDialog open={false} onClose={() => {}} onSubmit={vi.fn()} />);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("renders all fields when open", () => {
    render(<BrandKitDialog open={true} onClose={() => {}} onSubmit={vi.fn()} />);
    expect(screen.getByLabelText(/brand name/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/primary color/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/accent color/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^font$/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/destination directory/i)).toBeInTheDocument();
  });

  it("disables the Export button until brand name AND destination have content", async () => {
    const user = userEvent.setup();
    render(<BrandKitDialog open={true} onClose={() => {}} onSubmit={vi.fn()} />);
    const exportBtn = screen.getByRole("button", { name: /export/i });
    expect(exportBtn).toBeDisabled();

    await user.type(screen.getByLabelText(/brand name/i), "Acme");
    expect(exportBtn).toBeDisabled();

    await user.type(screen.getByLabelText(/destination directory/i), "/tmp/out");
    expect(exportBtn).toBeEnabled();
  });

  it("calls onSubmit with the typed values on Export click", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    const onClose = vi.fn();
    render(<BrandKitDialog open={true} onClose={onClose} onSubmit={onSubmit} />);

    await user.type(screen.getByLabelText(/brand name/i), "Acme");
    await user.type(screen.getByLabelText(/destination directory/i), "/tmp/out");

    await user.click(screen.getByRole("button", { name: /export/i }));

    expect(onSubmit).toHaveBeenCalledWith({
      brand_name: "Acme",
      primary_color: "#e85d2d",
      accent_color: "#0e0e11",
      font: "Inter",
      destination: "/tmp/out",
    });
    expect(onClose).toHaveBeenCalled();
  });

  it("displays an error alert when onSubmit rejects", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn().mockRejectedValue(new Error("boom"));
    render(<BrandKitDialog open={true} onClose={() => {}} onSubmit={onSubmit} />);
    await user.type(screen.getByLabelText(/brand name/i), "Acme");
    await user.type(screen.getByLabelText(/destination directory/i), "/tmp/out");
    await user.click(screen.getByRole("button", { name: /export/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/boom/);
  });

  it("calls onClose and resets fields when Cancel is clicked", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const onSubmit = vi.fn();
    const { rerender } = render(
      <BrandKitDialog open={true} onClose={onClose} onSubmit={onSubmit} />,
    );

    // Type into the fields, then cancel — onClose must fire.
    await user.type(screen.getByLabelText(/brand name/i), "Acme");
    await user.type(screen.getByLabelText(/destination directory/i), "/tmp/out");
    await user.click(screen.getByRole("button", { name: /cancel/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(onSubmit).not.toHaveBeenCalled();

    // Re-open the dialog — prior form state must be reset (empty inputs).
    rerender(<BrandKitDialog open={false} onClose={onClose} onSubmit={onSubmit} />);
    rerender(<BrandKitDialog open={true} onClose={onClose} onSubmit={onSubmit} />);
    expect(screen.getByLabelText(/brand name/i)).toHaveValue("");
    expect(screen.getByLabelText(/destination directory/i)).toHaveValue("");
  });

  it("propagates defaultBrandName and defaultDestination to the inputs", () => {
    render(
      <BrandKitDialog
        open={true}
        onClose={() => {}}
        onSubmit={vi.fn()}
        defaultBrandName="Preset"
        defaultDestination="/tmp/exports"
      />,
    );
    expect(screen.getByLabelText(/brand name/i)).toHaveValue("Preset");
    expect(screen.getByLabelText(/destination directory/i)).toHaveValue("/tmp/exports");
  });
});
