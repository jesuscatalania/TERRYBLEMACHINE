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

  it("renders the Typography page at /typography", async () => {
    renderAt("/typography");
    expect(await screen.findByText(/MOD—05 · TYPE & LOGO/)).toBeInTheDocument();
    expect(useAppStore.getState().activeModule).toBe("typography");
  });

  it("renders the Website builder at /website", async () => {
    renderAt("/website");
    expect(await screen.findByText(/WEBSITE BUILDER/)).toBeInTheDocument();
    expect(await screen.findByLabelText(/Describe the site/i)).toBeInTheDocument();
  });

  it("renders the design system page at /design-system", async () => {
    renderAt("/design-system");
    expect(await screen.findByRole("heading", { name: /^design system$/i })).toBeInTheDocument();
  });

  it("unknown routes fall back to /website (builder)", async () => {
    renderAt("/nonexistent");
    expect(await screen.findByText(/WEBSITE BUILDER/)).toBeInTheDocument();
  });

  it("renders the shell on every route", async () => {
    renderAt("/video");
    expect(await screen.findByText("TERRYBLEMACHINE")).toBeInTheDocument();
    expect(screen.getByRole("contentinfo")).toBeInTheDocument();
  });
});
