import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Sidebar } from "@/components/shell/Sidebar";
import { useAppStore } from "@/stores/appStore";

const navigateMock = vi.fn();
vi.mock("react-router-dom", async () => {
  const actual = await vi.importActual<typeof import("react-router-dom")>("react-router-dom");
  return {
    ...actual,
    useNavigate: () => navigateMock,
  };
});

function renderSidebar(path = "/website") {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Sidebar />
    </MemoryRouter>,
  );
}

describe("Sidebar", () => {
  beforeEach(() => {
    navigateMock.mockReset();
    useAppStore.setState({ theme: "dark", sidebarOpen: true, activeModule: "website" });
  });

  it("renders wordmark", () => {
    renderSidebar();
    expect(screen.getByText("TERRYBLEMACHINE")).toBeInTheDocument();
  });

  it("renders all 5 modules", () => {
    renderSidebar();
    for (const label of ["Website", "Graphic 2D", "Pseudo-3D", "Video", "Type & Logo"]) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
  });

  it("marks the active module with aria-current=page", () => {
    useAppStore.setState({ activeModule: "video" });
    renderSidebar();
    const active = screen
      .getAllByRole("button")
      .find((el) => el.getAttribute("aria-current") === "page");
    expect(active?.textContent).toContain("Video");
  });

  it("selecting another module updates the store", async () => {
    const user = userEvent.setup();
    renderSidebar();
    await user.click(screen.getByText("Pseudo-3D"));
    expect(useAppStore.getState().activeModule).toBe("graphic3d");
  });

  it("renders sections MODULES and PROJECT", () => {
    renderSidebar();
    expect(screen.getByText("Modules")).toBeInTheDocument();
    expect(screen.getByText("Project")).toBeInTheDocument();
  });

  it("clicking the collapse button toggles sidebarOpen in the store", async () => {
    const user = userEvent.setup();
    renderSidebar();
    const toggle = screen.getByRole("button", { name: /collapse sidebar|expand sidebar/i });
    await user.click(toggle);
    expect(useAppStore.getState().sidebarOpen).toBe(false);
  });

  it("renders a Chat with Claude button that navigates to /chat", async () => {
    const user = userEvent.setup();
    renderSidebar();
    const chatBtn = screen.getByRole("button", { name: /chat with claude/i });
    expect(chatBtn).toBeInTheDocument();
    await user.click(chatBtn);
    expect(navigateMock).toHaveBeenCalledWith("/chat");
  });
});
