import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Dropdown, type DropdownOption } from "@/components/ui/Dropdown";

const OPTIONS: DropdownOption[] = [
  { value: "claude", label: "Claude Opus" },
  { value: "kling", label: "Kling 2.0" },
  { value: "fal", label: "fal.ai Flux" },
  { value: "runway", label: "Runway Gen-3" },
];

describe("Dropdown", () => {
  it("renders the selected option label on the trigger", () => {
    render(<Dropdown options={OPTIONS} value="kling" onChange={() => {}} />);
    expect(screen.getByRole("combobox")).toHaveTextContent("Kling 2.0");
  });

  it("renders placeholder when no value", () => {
    render(<Dropdown options={OPTIONS} onChange={() => {}} placeholder="Choose model" />);
    expect(screen.getByRole("combobox")).toHaveTextContent("Choose model");
  });

  it("opens a listbox on click and lists all options", async () => {
    const user = userEvent.setup();
    render(<Dropdown options={OPTIONS} value="claude" onChange={() => {}} />);
    await user.click(screen.getByRole("combobox"));
    expect(screen.getByRole("listbox")).toBeInTheDocument();
    expect(screen.getAllByRole("option")).toHaveLength(4);
  });

  it("filters options by the search input", async () => {
    const user = userEvent.setup();
    render(<Dropdown options={OPTIONS} value="claude" onChange={() => {}} searchable />);
    await user.click(screen.getByRole("combobox"));
    await user.type(screen.getByRole("searchbox"), "kl");
    expect(screen.getAllByRole("option")).toHaveLength(1);
    expect(screen.getByRole("option")).toHaveTextContent("Kling 2.0");
  });

  it("calls onChange and closes when an option is picked", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Dropdown options={OPTIONS} value="claude" onChange={onChange} />);
    await user.click(screen.getByRole("combobox"));
    await user.click(screen.getByRole("option", { name: "fal.ai Flux" }));
    expect(onChange).toHaveBeenCalledWith("fal");
    expect(screen.queryByRole("listbox")).toBeNull();
  });
});
