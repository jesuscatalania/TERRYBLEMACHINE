import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "@/App";
import { useAppStore } from "@/stores/appStore";

// Monaco pulls in a Web Worker + language services that don't initialise in
// jsdom. Stub the component to a plain textarea so we can still exercise the
// builder page end-to-end in unit tests.
vi.mock("@monaco-editor/react", () => ({
  default: (props: { value?: string; onChange?: (v: string) => void }) => (
    <textarea
      data-testid="monaco-stub"
      defaultValue={props.value ?? ""}
      onChange={(e) => props.onChange?.(e.currentTarget.value)}
    />
  ),
}));

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

  it("redirects / → /website and renders the Website builder", async () => {
    renderAt("/");
    expect(await screen.findByText(/WEBSITE BUILDER/)).toBeInTheDocument();
    expect(useAppStore.getState().activeModule).toBe("website");
  });

  it.each([
    ["/typography", "Type & Logo", "typography"],
  ] as const)("renders %s placeholder and syncs store", (path, label, id) => {
    renderAt(path);
    expect(screen.getByText(new RegExp(`Coming soon — ${label}`))).toBeInTheDocument();
    expect(useAppStore.getState().activeModule).toBe(id);
  });

  it("renders the Website builder at /website", () => {
    renderAt("/website");
    expect(screen.getByText(/WEBSITE BUILDER/)).toBeInTheDocument();
    expect(screen.getByLabelText(/Describe the site/i)).toBeInTheDocument();
  });

  it("renders the design system page at /design-system", () => {
    renderAt("/design-system");
    expect(screen.getByRole("heading", { name: /^design system$/i })).toBeInTheDocument();
  });

  it("unknown routes fall back to /website (builder)", async () => {
    renderAt("/nonexistent");
    expect(await screen.findByText(/WEBSITE BUILDER/)).toBeInTheDocument();
  });

  it("renders the shell on every route", () => {
    renderAt("/video");
    expect(screen.getByText("TERRYBLEMACHINE")).toBeInTheDocument();
    expect(screen.getByRole("contentinfo")).toBeInTheDocument();
  });
});
