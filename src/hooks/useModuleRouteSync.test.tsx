import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "@/App";
import { useAppStore } from "@/stores/appStore";

// R3F uses WebGL which jsdom cannot provide. Stub the <Canvas> wrapper so
// navigating to /graphic3d doesn't trip the ResizeObserver/WebGL paths.
// NOTE: vi.mock is hoisted above imports, so stubs must be imported *inside*
// the factory — see src/test/r3f-mock-bodies.ts.
vi.mock("@react-three/fiber", async () => {
  const [actual, { FiberCanvasStub }] = await Promise.all([
    vi.importActual<typeof import("@react-three/fiber")>("@react-three/fiber"),
    import("@/test/r3f-mock-bodies"),
  ]);
  return { ...actual, Canvas: FiberCanvasStub };
});

vi.mock("@react-three/drei", async () => {
  const { DreiStubs } = await import("@/test/r3f-mock-bodies");
  return DreiStubs;
});

describe("module route sync", () => {
  beforeEach(() => {
    useAppStore.setState({ theme: "dark", sidebarOpen: true, activeModule: "website" });
  });

  it("sidebar click navigates AND updates store", async () => {
    const user = userEvent.setup();
    render(
      <MemoryRouter initialEntries={["/website"]}>
        <App />
      </MemoryRouter>,
    );
    expect(useAppStore.getState().activeModule).toBe("website");
    await user.click(screen.getByRole("button", { name: /Pseudo-3D/ }));
    expect(useAppStore.getState().activeModule).toBe("graphic3d");
    expect(await screen.findByText(/MOD—03 · PSEUDO-3D/)).toBeInTheDocument();
  });

  it("initial URL sets activeModule", () => {
    render(
      <MemoryRouter initialEntries={["/typography"]}>
        <App />
      </MemoryRouter>,
    );
    expect(useAppStore.getState().activeModule).toBe("typography");
  });
});
