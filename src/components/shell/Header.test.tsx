import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Header } from "@/components/shell/Header";
import { useAppStore } from "@/stores/appStore";

describe("Header", () => {
  beforeEach(() => {
    useAppStore.setState({ theme: "dark", sidebarOpen: true, activeModule: "website" });
  });

  it("renders breadcrumbs reflecting active module and project name", () => {
    render(<Header projectName="Untitled" onOpenSettings={() => {}} />);
    expect(screen.getByText("TM")).toBeInTheDocument();
    expect(screen.getByText("WEBSITE")).toBeInTheDocument();
    expect(screen.getByText("UNTITLED")).toBeInTheDocument();
  });

  it("reflects module changes", () => {
    useAppStore.setState({ activeModule: "typography" });
    render(<Header projectName="Untitled" onOpenSettings={() => {}} />);
    expect(screen.getByText("TYPE & LOGO")).toBeInTheDocument();
  });

  it("renders New and Generate buttons", () => {
    render(<Header projectName="Untitled" onOpenSettings={() => {}} />);
    expect(screen.getByRole("button", { name: /new/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /generate/i })).toBeInTheDocument();
  });

  it("calls onOpenSettings when settings button clicked", async () => {
    const user = userEvent.setup();
    const onOpenSettings = vi.fn();
    render(<Header projectName="Untitled" onOpenSettings={onOpenSettings} />);
    await user.click(screen.getByRole("button", { name: /settings/i }));
    expect(onOpenSettings).toHaveBeenCalledOnce();
  });
});
