import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

// Stub Monaco — the real editor spins up Web Workers jsdom can't run.
vi.mock("@monaco-editor/react", () => ({
  default: (props: { value?: string; onChange?: (v: string) => void }) => (
    <textarea
      data-testid="monaco-stub"
      defaultValue={props.value ?? ""}
      onChange={(e) => props.onChange?.(e.currentTarget.value)}
    />
  ),
}));

vi.mock("@/lib/websiteCommands", () => ({
  generateWebsite: vi.fn(async () => ({
    summary: "test-summary",
    prompt: "test",
    files: [{ path: "index.html", content: "<h1>Hi</h1>" }],
  })),
  analyzeUrl: vi.fn(async () => ({
    url: "https://stripe.com",
    status: 200,
    title: "Stripe",
    colors: [],
    fonts: [],
    spacing: [],
    customProperties: {},
    layout: "",
  })),
  exportWebsite: vi.fn(async () => "/tmp/export.zip"),
  modifyCodeSelection: vi.fn(async () => ({ replacement: "" })),
}));

vi.mock("@/lib/projectCommands", () => ({
  projectsRoot: vi.fn(async () => "/tmp/projects"),
  createProject: vi.fn(),
  openProjectFile: vi.fn(),
  listProjects: vi.fn().mockResolvedValue([]),
  deleteProject: vi.fn(),
  readProjectHistory: vi.fn().mockResolvedValue('{"past":[],"future":[]}'),
  writeProjectHistory: vi.fn().mockResolvedValue(undefined),
  isProjectIpcError: (v: unknown) => typeof v === "object" && v !== null && "kind" in v,
}));

import { generateWebsite } from "@/lib/websiteCommands";
import { WebsiteBuilderPage } from "@/pages/WebsiteBuilder";
import { useProjectStore } from "@/stores/projectStore";

function renderPage() {
  return render(
    <MemoryRouter>
      <WebsiteBuilderPage />
    </MemoryRouter>,
  );
}

describe("WebsiteBuilderPage", () => {
  beforeEach(() => {
    useProjectStore.setState({ currentProject: null, recents: [] });
    vi.mocked(generateWebsite).mockClear();
  });

  it("renders both the prompt textarea and the reference URL input", () => {
    renderPage();
    expect(screen.getByLabelText(/describe the site/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/reference url/i)).toBeInTheDocument();
  });

  it("disables the Generate button while the prompt is empty", () => {
    renderPage();
    expect(screen.getByRole("button", { name: /generate/i })).toBeDisabled();
  });

  it("enables Generate once a prompt is typed and calls generateWebsite on click", async () => {
    renderPage();
    fireEvent.change(screen.getByLabelText(/describe the site/i), {
      target: { value: "a landing page" },
    });
    const button = screen.getByRole("button", { name: /generate/i });
    expect(button).not.toBeDisabled();
    fireEvent.click(button);
    await waitFor(() => expect(generateWebsite).toHaveBeenCalledTimes(1));
    expect(vi.mocked(generateWebsite).mock.calls[0]?.[0]).toMatchObject({
      prompt: "a landing page",
      module: "website",
    });
  });

  it("keeps Export disabled until a project is generated", () => {
    renderPage();
    expect(screen.getByRole("button", { name: /export/i })).toBeDisabled();
  });

  it("enables Export after generateWebsite resolves with a project", async () => {
    renderPage();
    fireEvent.change(screen.getByLabelText(/describe the site/i), {
      target: { value: "shop for beans" },
    });
    fireEvent.click(screen.getByRole("button", { name: /generate/i }));
    await waitFor(() => expect(screen.getByRole("button", { name: /export/i })).not.toBeDisabled());
  });

  it("parses `/claude build me a blog` prompt: model_override=ClaudeSonnet, cleanPrompt=build me a blog", async () => {
    renderPage();
    fireEvent.change(screen.getByLabelText(/describe the site/i), {
      target: { value: "/claude build me a blog" },
    });
    fireEvent.click(screen.getByRole("button", { name: /^generate$/i }));
    await waitFor(() => expect(generateWebsite).toHaveBeenCalledTimes(1));
    expect(vi.mocked(generateWebsite).mock.calls[0]?.[0]).toMatchObject({
      prompt: "build me a blog",
      module: "website",
      model_override: "ClaudeSonnet",
    });
  });
});
