import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { Sidebar } from "@/components/shell/Sidebar";
import { useAppStore } from "@/stores/appStore";

describe("Sidebar", () => {
  beforeEach(() => {
    useAppStore.setState({ theme: "dark", sidebarOpen: true, activeModule: "website" });
  });

  it("renders wordmark", () => {
    render(<Sidebar />);
    expect(screen.getByText("TERRYBLEMACHINE")).toBeInTheDocument();
  });

  it("renders all 5 modules", () => {
    render(<Sidebar />);
    for (const label of ["Website", "Graphic 2D", "Pseudo-3D", "Video", "Type & Logo"]) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
  });

  it("marks the active module with aria-current=page", () => {
    useAppStore.setState({ activeModule: "video" });
    render(<Sidebar />);
    const active = screen
      .getAllByRole("button")
      .find((el) => el.getAttribute("aria-current") === "page");
    expect(active?.textContent).toContain("Video");
  });

  it("selecting another module updates the store", async () => {
    const user = userEvent.setup();
    render(<Sidebar />);
    await user.click(screen.getByText("Pseudo-3D"));
    expect(useAppStore.getState().activeModule).toBe("graphic3d");
  });

  it("renders sections MODULES and PROJECT", () => {
    render(<Sidebar />);
    expect(screen.getByText("Modules")).toBeInTheDocument();
    expect(screen.getByText("Project")).toBeInTheDocument();
  });

  it("clicking the collapse button toggles sidebarOpen in the store", async () => {
    const user = userEvent.setup();
    render(<Sidebar />);
    const toggle = screen.getByRole("button", { name: /collapse sidebar|expand sidebar/i });
    await user.click(toggle);
    expect(useAppStore.getState().sidebarOpen).toBe(false);
  });
});
