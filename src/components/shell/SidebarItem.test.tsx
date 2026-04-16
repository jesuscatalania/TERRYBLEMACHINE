import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SidebarItem } from "@/components/shell/SidebarItem";

describe("SidebarItem", () => {
  it("renders label, index and kbd hint", () => {
    render(
      <SidebarItem
        moduleId="website"
        label="Website"
        index="01"
        shortcut="⌘1"
        active={false}
        onSelect={() => {}}
      />,
    );
    expect(screen.getByText("Website")).toBeInTheDocument();
    expect(screen.getByText("/ 01")).toBeInTheDocument();
    expect(screen.getByText("⌘1")).toBeInTheDocument();
  });

  it("marks aria-current=page when active", () => {
    render(
      <SidebarItem
        moduleId="website"
        label="Website"
        index="01"
        shortcut="⌘1"
        active={true}
        onSelect={() => {}}
      />,
    );
    expect(screen.getByRole("button")).toHaveAttribute("aria-current", "page");
  });

  it("invokes onSelect when clicked", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(
      <SidebarItem
        moduleId="video"
        label="Video"
        index="04"
        shortcut="⌘4"
        active={false}
        onSelect={onSelect}
      />,
    );
    await user.click(screen.getByRole("button"));
    expect(onSelect).toHaveBeenCalledWith("video");
  });
});
