import { beforeEach, describe, expect, it, vi } from "vitest";
import { useHistoryStore } from "@/stores/historyStore";
import { useProjectStore } from "@/stores/projectStore";

// History persistence rides on Tauri IPC — stub it out so store unit tests
// don't hit the bridge. Returns the empty-stacks payload on read; resolves
// writes silently.
vi.mock("@/lib/projectCommands", () => ({
  readProjectHistory: vi.fn().mockResolvedValue('{"past":[],"future":[]}'),
  writeProjectHistory: vi.fn().mockResolvedValue(undefined),
}));

describe("projectStore", () => {
  beforeEach(() => {
    useProjectStore.setState({ currentProject: null, recents: [] });
    useHistoryStore.setState({ past: [], future: [] });
  });

  it("starts with no open project", () => {
    expect(useProjectStore.getState().currentProject).toBeNull();
  });

  it("opens a project and records it as current", () => {
    const project = {
      id: "p1",
      name: "Demo",
      module: "website" as const,
      path: "/tmp/demo",
      createdAt: "2026-04-16T10:00:00Z",
    };
    useProjectStore.getState().openProject(project);
    expect(useProjectStore.getState().currentProject).toEqual(project);
  });

  it("closing a project clears current", () => {
    useProjectStore.getState().openProject({
      id: "p1",
      name: "Demo",
      module: "website",
      path: "/tmp/demo",
      createdAt: "2026-04-16T10:00:00Z",
    });
    useProjectStore.getState().closeProject();
    expect(useProjectStore.getState().currentProject).toBeNull();
  });

  it("adds a project to recents without duplicates, most recent first", () => {
    const p1 = {
      id: "1",
      name: "A",
      module: "website" as const,
      path: "/a",
      createdAt: "2026-01-01",
    };
    const p2 = {
      id: "2",
      name: "B",
      module: "video" as const,
      path: "/b",
      createdAt: "2026-01-02",
    };
    useProjectStore.getState().addRecent(p1);
    useProjectStore.getState().addRecent(p2);
    useProjectStore.getState().addRecent(p1);
    const recents = useProjectStore.getState().recents;
    expect(recents.map((p) => p.id)).toEqual(["1", "2"]);
  });

  it("caps recents at 10 entries", () => {
    for (let i = 0; i < 15; i++) {
      useProjectStore.getState().addRecent({
        id: `p${i}`,
        name: `P${i}`,
        module: "website",
        path: `/p${i}`,
        createdAt: "2026-04-16",
      });
    }
    expect(useProjectStore.getState().recents).toHaveLength(10);
  });

  it("hydrateRecents replaces the list and caps at 10", () => {
    const projects = Array.from({ length: 15 }, (_, i) => ({
      id: `p${i}`,
      name: `P${i}`,
      module: "website" as const,
      path: `/p${i}`,
      createdAt: "2026-04-16",
    }));
    useProjectStore.getState().hydrateRecents(projects);
    expect(useProjectStore.getState().recents).toHaveLength(10);
    expect(useProjectStore.getState().recents[0]?.id).toBe("p0");
  });

  it("opening a project clears the undo/redo history", () => {
    useHistoryStore.getState().push({
      label: "demo",
      do: () => {},
      undo: () => {},
    });
    expect(useHistoryStore.getState().past).toHaveLength(1);
    useProjectStore.getState().openProject({
      id: "p1",
      name: "Demo",
      module: "website",
      path: "/tmp/demo",
      createdAt: "2026-04-17T00:00:00Z",
    });
    expect(useHistoryStore.getState().past).toHaveLength(0);
  });

  it("closing a project clears the undo/redo history", () => {
    useHistoryStore.getState().push({
      label: "demo",
      do: () => {},
      undo: () => {},
    });
    useProjectStore.getState().closeProject();
    expect(useHistoryStore.getState().past).toHaveLength(0);
  });
});
