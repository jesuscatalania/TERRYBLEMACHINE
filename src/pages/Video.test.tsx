import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

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

import { VideoPage } from "@/pages/Video";

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
});
