import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Input, NumberInput, Textarea } from "@/components/ui/Input";

describe("Input", () => {
  it("renders a text input with placeholder", () => {
    render(<Input placeholder="Search…" />);
    expect(screen.getByPlaceholderText("Search…")).toBeInTheDocument();
  });

  it("emits onChange with the typed value", async () => {
    const user = userEvent.setup();
    const handler = vi.fn<(value: string) => void>();
    render(<Input onValueChange={handler} />);
    await user.type(screen.getByRole("textbox"), "ab");
    expect(handler).toHaveBeenLastCalledWith("ab");
  });

  it("supports a leading label", () => {
    render(<Input label="Name" id="name-input" />);
    expect(screen.getByLabelText("Name")).toBeInTheDocument();
  });

  it("shows error message when error prop is set", () => {
    render(<Input label="Name" id="n" error="Required" />);
    expect(screen.getByText("Required")).toBeInTheDocument();
  });
});

describe("Textarea", () => {
  it("renders a textarea", () => {
    render(<Textarea placeholder="Prompt" />);
    expect(screen.getByPlaceholderText("Prompt")).toBeInTheDocument();
  });

  it("auto-resizes on input (sets style.height)", async () => {
    const user = userEvent.setup();
    render(<Textarea defaultValue="" />);
    const ta = screen.getByRole("textbox") as HTMLTextAreaElement;
    await user.type(ta, "line1\nline2\nline3");
    // auto-resize sets inline height > 0
    expect(ta.style.height).not.toBe("");
  });
});

describe("NumberInput", () => {
  it("renders with inputmode=numeric", () => {
    render(<NumberInput placeholder="0" />);
    const el = screen.getByPlaceholderText("0");
    expect(el).toHaveAttribute("inputmode", "numeric");
  });
});
