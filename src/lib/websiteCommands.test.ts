import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

import type { GeneratedProject } from "@/lib/websiteCommands";
import { openInBrowser, refineWebsite } from "@/lib/websiteCommands";

const seedProject: GeneratedProject = {
  summary: "seed",
  prompt: "seed prompt",
  files: [{ path: "index.html", content: "<h1>Hi</h1>" }],
};

beforeEach(() => {
  vi.clearAllMocks();
});

describe("refineWebsite", () => {
  it("invokes refine_website with { input: { project, instruction } } shape", async () => {
    invoke.mockResolvedValueOnce({
      project: {
        summary: "refined",
        prompt: "seed prompt",
        files: [{ path: "index.html", content: "<h1>Updated</h1>" }],
      },
      changed_paths: ["index.html"],
    });
    const result = await refineWebsite(seedProject, "make it better");
    expect(invoke).toHaveBeenCalledWith("refine_website", {
      input: { project: seedProject, instruction: "make it better" },
    });
    expect(result.changed_paths).toEqual(["index.html"]);
    expect(result.project.summary).toBe("refined");
  });

  it("propagates rejections from the invoke layer unchanged", async () => {
    invoke.mockRejectedValueOnce({ kind: "InvalidInput", detail: "instruction is empty" });
    await expect(refineWebsite(seedProject, "")).rejects.toMatchObject({
      kind: "InvalidInput",
    });
  });
});

describe("openInBrowser", () => {
  it("invokes open_project_in_browser with { project } shape", async () => {
    invoke.mockResolvedValueOnce("file:///tmp/tm-preview-abc/index.html");
    const url = await openInBrowser(seedProject);
    expect(invoke).toHaveBeenCalledWith("open_project_in_browser", {
      project: seedProject,
    });
    expect(url).toContain("file://");
  });
});
