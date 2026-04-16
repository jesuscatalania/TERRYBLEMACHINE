import { beforeEach, describe, expect, it } from "vitest";
import { useAppStore } from "@/stores/appStore";

describe("appStore", () => {
  beforeEach(() => {
    useAppStore.setState({
      theme: "dark",
      sidebarOpen: true,
      activeModule: "website",
    });
  });

  it("has dark theme as default", () => {
    expect(useAppStore.getState().theme).toBe("dark");
  });

  it("has sidebar open by default", () => {
    expect(useAppStore.getState().sidebarOpen).toBe(true);
  });

  it("toggles sidebar", () => {
    useAppStore.getState().toggleSidebar();
    expect(useAppStore.getState().sidebarOpen).toBe(false);
    useAppStore.getState().toggleSidebar();
    expect(useAppStore.getState().sidebarOpen).toBe(true);
  });

  it("switches active module", () => {
    useAppStore.getState().setActiveModule("video");
    expect(useAppStore.getState().activeModule).toBe("video");
  });

  it("switches theme", () => {
    useAppStore.getState().setTheme("light");
    expect(useAppStore.getState().theme).toBe("light");
  });
});
