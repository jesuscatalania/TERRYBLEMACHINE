import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";
import App from "@/App";

function renderAt(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <App />
    </MemoryRouter>,
  );
}

describe("App", () => {
  it("renders the home page at /", () => {
    renderAt("/");
    expect(screen.getByRole("heading", { name: /describe what to build/i })).toBeInTheDocument();
  });

  it("renders the design system page at /design-system", () => {
    renderAt("/design-system");
    expect(screen.getByRole("heading", { name: /design system/i })).toBeInTheDocument();
  });

  it("renders the shell (wordmark + status bar) on any route", () => {
    renderAt("/design-system");
    expect(screen.getByText("TERRYBLEMACHINE")).toBeInTheDocument();
    expect(screen.getByRole("contentinfo")).toBeInTheDocument();
  });
});
