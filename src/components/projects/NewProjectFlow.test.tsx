import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "@/App";
import { useProjectStore } from "@/stores/projectStore";

vi.mock("@/lib/projectCommands", () => ({
  createProject: vi.fn().mockImplementation(async (input) => ({
    id: "my-project",
    name: input.name,
    module: input.module,
    path: "/tmp/my-project",
    createdAt: "2026-04-16T10:00:00Z",
    description: input.description,
  })),
  listProjects: vi.fn().mockResolvedValue([]),
  openProjectFile: vi.fn(),
  deleteProject: vi.fn(),
  projectsRoot: vi.fn().mockResolvedValue("/tmp"),
  isProjectIpcError: (v: unknown) => typeof v === "object" && v !== null && "kind" in v,
}));

describe("New Project flow", () => {
  beforeEach(() => {
    useProjectStore.setState({ currentProject: null, recents: [] });
  });

  it("clicking NEW opens the dialog, submitting creates + opens the project", async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={["/website"]}>
        <App />
      </MemoryRouter>,
    );

    await user.click(screen.getByRole("button", { name: /^new$/i }));
    expect(await screen.findByRole("dialog")).toBeInTheDocument();

    await user.type(screen.getByLabelText(/name/i), "My Project");
    await user.click(screen.getByRole("button", { name: /create/i }));

    await waitFor(() => {
      expect(useProjectStore.getState().currentProject?.name).toBe("My Project");
    });

    // Dialog closed after success
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });
});
