import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { Shell } from "@/components/shell/Shell";
import { useAppStore } from "@/stores/appStore";

describe("Shell", () => {
  beforeEach(() => {
    useAppStore.setState({ theme: "dark", sidebarOpen: true, activeModule: "website" });
  });

  it("renders sidebar, header, footer and children", () => {
    render(
      <Shell>
        <p>main-content</p>
      </Shell>,
    );
    expect(screen.getByText("TERRYBLEMACHINE")).toBeInTheDocument();
    expect(screen.getByRole("banner")).toBeInTheDocument();
    expect(screen.getByRole("contentinfo")).toBeInTheDocument();
    expect(screen.getByText("main-content")).toBeInTheDocument();
  });

  it("renders the main landmark with the project content", () => {
    render(
      <Shell>
        <div data-testid="child" />
      </Shell>,
    );
    expect(screen.getByRole("main")).toContainElement(screen.getByTestId("child"));
  });
});
