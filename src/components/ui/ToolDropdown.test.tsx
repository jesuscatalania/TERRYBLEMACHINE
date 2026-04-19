import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ToolDropdown } from "@/components/ui/ToolDropdown";

describe("ToolDropdown", () => {
  it("renders selected label", () => {
    render(<ToolDropdown taskKind="ImageGeneration" value="auto" onChange={() => {}} />);
    expect(screen.getByRole("button", { name: /Auto/i })).toBeInTheDocument();
  });

  it("opens menu and lists all catalog entries grouped by tier", async () => {
    const user = userEvent.setup();
    render(<ToolDropdown taskKind="TextToVideo" value="auto" onChange={() => {}} />);
    await user.click(screen.getByRole("button", { name: /Auto/i }));
    // "Auto" appears on both the trigger and the menu option; use the
    // more specific "router decides" phrase to pin the menu option.
    expect(screen.getByText(/Auto \(router decides\)/i)).toBeInTheDocument();
    expect(screen.getByText(/Kling V2 Master/i)).toBeInTheDocument();
    expect(screen.getByText(/Kling V1\.5/i)).toBeInTheDocument();
    expect(screen.getByText(/Runway Gen-3/i)).toBeInTheDocument();
    expect(screen.getByText(/Higgsfield/i)).toBeInTheDocument();
  });

  it("onChange fires with the chosen Model name", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ToolDropdown taskKind="ImageGeneration" value="auto" onChange={onChange} />);
    await user.click(screen.getByRole("button"));
    await user.click(screen.getByText(/SDXL Fast/i));
    expect(onChange).toHaveBeenCalledWith("FalSdxl");
  });

  it("Auto option fires onChange with 'auto'", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ToolDropdown taskKind="ImageGeneration" value="FalFluxPro" onChange={onChange} />);
    await user.click(screen.getByRole("button"));
    await user.click(screen.getByText(/Auto \(router decides\)/i));
    expect(onChange).toHaveBeenCalledWith("auto");
  });
});
