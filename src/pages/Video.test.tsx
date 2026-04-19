import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

// Video page uses Button / Input / video store + storyboard/video/assembly commands.
// Stub the commands so the page renders without hitting Tauri.
vi.mock("@/lib/storyboardCommands", () => ({
  generateStoryboard: vi.fn(async () => ({
    summary: "s",
    template: "commercial",
    shots: [
      {
        index: 1,
        description: "x",
        style: "",
        duration_s: 5,
        camera: "",
        transition: "",
      },
    ],
  })),
}));
vi.mock("@/lib/videoCommands", () => ({
  generateVideoFromText: vi.fn(),
  generateVideoFromImage: vi.fn(),
}));
vi.mock("@/lib/remotionCommands", () => ({
  renderRemotion: vi.fn(),
}));
vi.mock("@/lib/assemblyCommands", () => ({
  assembleVideo: vi.fn(),
}));

import { generateStoryboard } from "@/lib/storyboardCommands";
import { generateVideoFromText } from "@/lib/videoCommands";
import { VideoPage } from "@/pages/Video";
import { useVideoStore } from "@/stores/videoStore";

afterEach(() => {
  vi.clearAllMocks();
  useVideoStore.getState().reset();
});

describe("VideoPage", () => {
  it("renders the module banner", () => {
    render(
      <MemoryRouter>
        <VideoPage />
      </MemoryRouter>,
    );
    expect(screen.getByText(/MOD—04/)).toBeInTheDocument();
    expect(screen.getByText(/MOD—04 · VIDEO/)).toBeInTheDocument();
  });

  it("shows the storyboard brief input", () => {
    render(
      <MemoryRouter>
        <VideoPage />
      </MemoryRouter>,
    );
    expect(screen.getByLabelText(/describe the video/i)).toBeInTheDocument();
  });

  it("shows the template dropdown", () => {
    render(
      <MemoryRouter>
        <VideoPage />
      </MemoryRouter>,
    );
    expect(screen.getByLabelText(/template/i)).toBeInTheDocument();
  });

  it("shows empty storyboard state initially", () => {
    render(
      <MemoryRouter>
        <VideoPage />
      </MemoryRouter>,
    );
    expect(screen.getByText(/No storyboard yet/i)).toBeInTheDocument();
  });

  it("shows empty segment list initially", () => {
    render(
      <MemoryRouter>
        <VideoPage />
      </MemoryRouter>,
    );
    expect(screen.getByText(/No segments yet/i)).toBeInTheDocument();
  });

  it("double-click on Generate segments does not double-fire the IPC (re-entrancy guard)", async () => {
    const user = userEvent.setup();
    // Hold the in-flight promise so we can simulate a slow backend and
    // fire the second click while the first call is still pending. Using
    // a holder object so TypeScript doesn't narrow the closure-assigned
    // field to `never` at the read site.
    const control: {
      resolve:
        | ((v: {
            video_url: string;
            local_path: string | null;
            model: string;
            duration_s: number | null;
          }) => void)
        | null;
    } = { resolve: null };
    vi.mocked(generateVideoFromText).mockImplementation(
      () =>
        new Promise<{
          video_url: string;
          local_path: string | null;
          model: string;
          duration_s: number | null;
        }>((resolve) => {
          control.resolve = resolve;
        }),
    );

    // Seed a single AI segment so Generate segments has something to loop over.
    useVideoStore.getState().addSegment({
      kind: "ai",
      label: "Opening shot",
      duration_s: 5,
    });

    render(
      <MemoryRouter>
        <VideoPage />
      </MemoryRouter>,
    );

    const btn = screen.getByRole("button", { name: /generate segments/i });
    await user.click(btn);
    // Second click while the first is still in flight — must be ignored
    // by the busy guard. Double-click without the guard would fire
    // `generateVideoFromText` twice for the same segment.
    await user.click(btn);
    expect(generateVideoFromText).toHaveBeenCalledTimes(1);

    // Let the call resolve so the finally clears state cleanly.
    control.resolve?.({
      video_url: "file:///tmp/seg.mp4",
      local_path: "/tmp/seg.mp4",
      model: "kling",
      duration_s: 5,
    });
    await waitFor(() => {
      const seg = useVideoStore.getState().segments[0];
      expect(seg?.video_url).toBe("file:///tmp/seg.mp4");
    });
  });

  it("parses `/kling sunrise timelapse` prompt: model_override=FalKlingV2Master, cleanPrompt=sunrise timelapse", async () => {
    render(
      <MemoryRouter>
        <VideoPage />
      </MemoryRouter>,
    );
    fireEvent.change(screen.getByLabelText(/describe the video/i), {
      target: { value: "/kling sunrise timelapse" },
    });
    fireEvent.click(screen.getByRole("button", { name: /generate storyboard/i }));
    await waitFor(() => expect(generateStoryboard).toHaveBeenCalledTimes(1));
    expect(vi.mocked(generateStoryboard).mock.calls[0]?.[0]).toMatchObject({
      prompt: "sunrise timelapse",
      module: "video",
      model_override: "FalKlingV2Master",
    });
  });
});
