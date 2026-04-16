import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it } from "vitest";
import App from "@/App";
import { useAppStore } from "@/stores/appStore";

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
    expect(await screen.findByText(/Coming soon — Pseudo-3D/)).toBeInTheDocument();
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
