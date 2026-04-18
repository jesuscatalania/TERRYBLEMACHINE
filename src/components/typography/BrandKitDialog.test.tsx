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
});
