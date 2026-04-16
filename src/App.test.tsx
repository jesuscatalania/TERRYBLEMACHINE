import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it } from "vitest";
import App from "@/App";
import { useAppStore } from "@/stores/appStore";

function renderAt(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <App />
    </MemoryRouter>,
  );
}

describe("App", () => {
  beforeEach(() => {
    useAppStore.setState({ theme: "dark", sidebarOpen: true, activeModule: "website" });
  });

  it("redirects / → /website and renders the Website placeholder", async () => {
    renderAt("/");
    expect(await screen.findByText(/Coming soon — Website/)).toBeInTheDocument();
    expect(useAppStore.getState().activeModule).toBe("website");
  });

  it.each([
    ["/website", "Website", "website"],
    ["/graphic2d", "Graphic 2D", "graphic2d"],
    ["/graphic3d", "Pseudo-3D", "graphic3d"],
    ["/video", "Video", "video"],
    ["/typography", "Type & Logo", "typography"],
  ] as const)("renders %s placeholder and syncs store", (path, label, id) => {
    renderAt(path);
    expect(screen.getByText(new RegExp(`Coming soon — ${label}`))).toBeInTheDocument();
    expect(useAppStore.getState().activeModule).toBe(id);
  });

  it("renders the design system page at /design-system", () => {
    renderAt("/design-system");
    expect(screen.getByRole("heading", { name: /^design system$/i })).toBeInTheDocument();
  });

  it("unknown routes fall back to /website", async () => {
    renderAt("/nonexistent");
    expect(await screen.findByText(/Coming soon — Website/)).toBeInTheDocument();
  });

  it("renders the shell on every route", () => {
    renderAt("/video");
    expect(screen.getByText("TERRYBLEMACHINE")).toBeInTheDocument();
    expect(screen.getByRole("contentinfo")).toBeInTheDocument();
  });
});
