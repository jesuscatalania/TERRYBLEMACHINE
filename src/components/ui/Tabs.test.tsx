import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Globe, Image } from "lucide-react";
import { describe, expect, it, vi } from "vitest";
import { type TabItem, Tabs } from "@/components/ui/Tabs";

const ITEMS: TabItem[] = [
  { id: "website", label: "Website", icon: <Globe /> },
  { id: "graphic", label: "Graphic", icon: <Image /> },
  { id: "disabled", label: "Off", disabled: true },
];

describe("Tabs", () => {
  it("renders all items", () => {
    render(<Tabs items={ITEMS} activeId="website" onChange={() => {}} />);
    expect(screen.getByRole("tab", { name: /website/i })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /graphic/i })).toBeInTheDocument();
  });

  it("marks the active tab with aria-selected=true", () => {
    render(<Tabs items={ITEMS} activeId="graphic" onChange={() => {}} />);
    expect(screen.getByRole("tab", { name: /graphic/i })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tab", { name: /website/i })).toHaveAttribute("aria-selected", "false");
  });

  it("calls onChange when an item is clicked", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Tabs items={ITEMS} activeId="website" onChange={onChange} />);
    await user.click(screen.getByRole("tab", { name: /graphic/i }));
    expect(onChange).toHaveBeenCalledWith("graphic");
  });

  it("does not fire onChange for disabled items", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<Tabs items={ITEMS} activeId="website" onChange={onChange} />);
    await user.click(screen.getByRole("tab", { name: /off/i }));
    expect(onChange).not.toHaveBeenCalled();
  });
});
