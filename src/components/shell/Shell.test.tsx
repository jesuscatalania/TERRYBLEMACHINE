import { render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it } from "vitest";
import { Shell } from "@/components/shell/Shell";
import { useAppStore } from "@/stores/appStore";

function renderShell(children: ReactNode) {
  return render(
    <MemoryRouter initialEntries={["/website"]}>
      <Shell>{children}</Shell>
    </MemoryRouter>,
  );
}

describe("Shell", () => {
  beforeEach(() => {
    useAppStore.setState({ theme: "dark", sidebarOpen: true, activeModule: "website" });
  });

  it("renders sidebar, header, footer and children", () => {
    renderShell(<p>main-content</p>);
    expect(screen.getByText("TERRYBLEMACHINE")).toBeInTheDocument();
    expect(screen.getByRole("banner")).toBeInTheDocument();
    expect(screen.getByRole("contentinfo")).toBeInTheDocument();
    expect(screen.getByText("main-content")).toBeInTheDocument();
  });

  it("renders the main landmark with the project content", () => {
    renderShell(<div data-testid="child" />);
    expect(screen.getByRole("main")).toContainElement(screen.getByTestId("child"));
  });

  it("uses the expanded grid when sidebarOpen is true", () => {
    const { container } = renderShell(<div data-testid="content">content</div>);
    const grid = container.querySelector("[data-testid='shell-grid']");
    expect(grid?.className).toMatch(/grid-cols-\[15rem_1fr\]/);
  });

  it("uses the collapsed grid when sidebarOpen is false", () => {
    useAppStore.setState({ sidebarOpen: false });
    const { container } = renderShell(<div data-testid="content">content</div>);
    const grid = container.querySelector("[data-testid='shell-grid']");
    expect(grid?.className).toMatch(/grid-cols-\[3\.5rem_1fr\]/);
  });
});
