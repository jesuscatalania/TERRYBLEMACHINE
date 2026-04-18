import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

// Mock the three *Commands.ts wrappers rather than `@tauri-apps/api/core`'s
// `invoke` directly — matches the pattern used elsewhere in the test suite
// (the IPC shape is an implementation detail of the wrappers). Keeps the
// test focused on the page-level flow: Generate → select → Vectorize →
// Export button enables → Export dialog submit → success toast.
vi.mock("@/lib/logoCommands", async () => {
  const actual = await vi.importActual<typeof import("@/lib/logoCommands")>("@/lib/logoCommands");
  return {
    ...actual,
    generateLogoVariants: vi.fn(),
  };
});

vi.mock("@/lib/vectorizerCommands", async () => {
  const actual = await vi.importActual<typeof import("@/lib/vectorizerCommands")>(
    "@/lib/vectorizerCommands",
  );
  return {
    ...actual,
    vectorizeImage: vi.fn(),
  };
});

vi.mock("@/lib/brandKitCommands", async () => {
  const actual =
    await vi.importActual<typeof import("@/lib/brandKitCommands")>("@/lib/brandKitCommands");
  return {
    ...actual,
    exportBrandKit: vi.fn(),
  };
});

import { exportBrandKit } from "@/lib/brandKitCommands";
import { generateLogoVariants, type LogoVariant } from "@/lib/logoCommands";
import { vectorizeImage } from "@/lib/vectorizerCommands";
import { TypographyPage } from "@/pages/Typography";
import { useLogoStore } from "@/stores/logoStore";
import { useUiStore } from "@/stores/uiStore";

afterEach(() => {
  vi.clearAllMocks();
  useUiStore.setState({ notifications: [] });
  useLogoStore.getState().clearFavorites();
});

describe("TypographyPage integration", () => {
  it("walks Generate → select → Vectorize → Export → brand kit success toast", async () => {
    const user = userEvent.setup();
    const variants: LogoVariant[] = [
      { url: "https://x/a.png", local_path: "/tmp/a.png", seed: 1, model: "ideogram-v3" },
      { url: "https://x/b.png", local_path: "/tmp/b.png", seed: 2, model: "ideogram-v3" },
    ];
    vi.mocked(generateLogoVariants).mockResolvedValue(variants);
    vi.mocked(vectorizeImage).mockResolvedValue({
      svg: '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"/>',
      width: 10,
      height: 10,
    });
    vi.mocked(exportBrandKit).mockResolvedValue("/tmp/out/acme-brand-kit.zip");

    render(
      <MemoryRouter>
        <TypographyPage />
      </MemoryRouter>,
    );

    // 1) Generate — type a prompt + click the button.
    await user.type(screen.getByLabelText(/describe the logo/i), "Acme mark");
    await user.click(screen.getByRole("button", { name: /generate 6 variants/i }));
    await waitFor(() => expect(generateLogoVariants).toHaveBeenCalledTimes(1));

    // 2) Select the first variant — the Vectorize button should appear.
    // Two "Select logo variant" buttons render (one per variant); the
    // first one corresponds to "https://x/a.png" because React renders
    // in array order.
    await screen.findByTestId("logo-variant-https://x/a.png");
    const [firstSelectBtn] = screen.getAllByRole("button", { name: /select logo variant/i });
    await user.click(firstSelectBtn);

    // 3) Vectorize — the Export button is gated on `vectorized` + local_path.
    const exportBtn = screen.getByRole("button", { name: /export brand kit/i });
    expect(exportBtn).toBeDisabled();
    await user.click(screen.getByRole("button", { name: /^vectorize$/i }));
    await waitFor(() => expect(vectorizeImage).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(exportBtn).toBeEnabled());

    // 4) Open the dialog, fill in + submit.
    await user.click(exportBtn);
    await user.type(screen.getByLabelText(/brand name/i), "Acme");
    await user.type(screen.getByLabelText(/destination directory/i), "/tmp/out");
    await user.click(screen.getByRole("button", { name: /^export$/i }));

    await waitFor(() => expect(exportBrandKit).toHaveBeenCalledTimes(1));
    // Assert the success toast landed in the uiStore with the zip path.
    await waitFor(() => {
      const { notifications } = useUiStore.getState();
      expect(notifications.some((n) => /brand kit exported/i.test(n.message))).toBe(true);
    });
  });
});
