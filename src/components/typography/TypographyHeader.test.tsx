import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { TypographyHeader } from "@/components/typography/TypographyHeader";

describe("TypographyHeader", () => {
  const props = {
    prompt: "",
    onPromptChange: vi.fn(),
    style: "minimalist" as const,
    onStyleChange: vi.fn(),
    palette: "",
    onPaletteChange: vi.fn(),
    busy: false,
    onGenerate: vi.fn(),
  };

  it("renders the MOD—05 tag and all four inputs", () => {
    render(<TypographyHeader {...props} />);
    expect(screen.getByText(/MOD—05 · TYPE & LOGO/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Describe the logo/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Logo style/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Palette/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Generate 6 variants/i })).toBeInTheDocument();
  });

  it("fires onGenerate when the button is clicked with a non-empty prompt", async () => {
    const user = userEvent.setup();
    const onGenerate = vi.fn();
    render(<TypographyHeader {...props} prompt="hello" onGenerate={onGenerate} />);
    await user.click(screen.getByRole("button", { name: /Generate/i }));
    expect(onGenerate).toHaveBeenCalledTimes(1);
  });

  it("disables the Generate button when prompt is empty OR busy", () => {
    const { rerender } = render(<TypographyHeader {...props} prompt="" />);
    expect(screen.getByRole("button", { name: /Generate/i })).toBeDisabled();
    rerender(<TypographyHeader {...props} prompt="hello" busy={true} />);
    expect(screen.getByRole("button", { name: /Generate/i })).toBeDisabled();
    rerender(<TypographyHeader {...props} prompt="hello" busy={false} />);
    expect(screen.getByRole("button", { name: /Generate/i })).toBeEnabled();
  });

  it("shows the loading spinner while busy (Generate label stays constant)", () => {
    render(<TypographyHeader {...props} prompt="hello" busy={true} />);
    expect(screen.getByRole("button", { name: /Generate 6 variants/i })).toBeInTheDocument();
    expect(screen.getByTestId("loading-spinner")).toBeInTheDocument();
  });
});
