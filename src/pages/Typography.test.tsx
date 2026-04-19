import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/logoCommands", () => ({
  generateLogoVariants: vi.fn(async () => []),
}));

import { generateLogoVariants } from "@/lib/logoCommands";
import { TypographyPage } from "@/pages/Typography";

describe("TypographyPage", () => {
  beforeEach(() => {
    vi.mocked(generateLogoVariants).mockClear();
  });

  it("renders the module banner", () => {
    render(
      <MemoryRouter>
        <TypographyPage />
      </MemoryRouter>,
    );
    expect(screen.getByText(/MOD—05/)).toBeInTheDocument();
    expect(screen.getByText(/TYPE & LOGO/i)).toBeInTheDocument();
  });

  it("shows prompt input", () => {
    render(
      <MemoryRouter>
        <TypographyPage />
      </MemoryRouter>,
    );
    expect(screen.getByLabelText(/describe the logo/i)).toBeInTheDocument();
  });

  it("shows style dropdown", () => {
    render(
      <MemoryRouter>
        <TypographyPage />
      </MemoryRouter>,
    );
    expect(screen.getByLabelText(/logo style/i)).toBeInTheDocument();
  });

  it("shows palette input", () => {
    render(
      <MemoryRouter>
        <TypographyPage />
      </MemoryRouter>,
    );
    expect(screen.getByLabelText(/palette/i)).toBeInTheDocument();
  });

  it("shows empty gallery initially", () => {
    render(
      <MemoryRouter>
        <TypographyPage />
      </MemoryRouter>,
    );
    expect(screen.getByText(/No logos yet/i)).toBeInTheDocument();
  });

  it("parses `/ideogram bold sans` prompt: model_override=IdeogramV3, cleanPrompt=bold sans", async () => {
    render(
      <MemoryRouter>
        <TypographyPage />
      </MemoryRouter>,
    );
    fireEvent.change(screen.getByLabelText(/describe the logo/i), {
      target: { value: "/ideogram bold sans" },
    });
    fireEvent.click(screen.getByRole("button", { name: /generate 6 variants/i }));
    await waitFor(() => expect(generateLogoVariants).toHaveBeenCalledTimes(1));
    expect(vi.mocked(generateLogoVariants).mock.calls[0]?.[0]).toMatchObject({
      prompt: "bold sans",
      module: "typography",
      model_override: "IdeogramV3",
    });
  });
});
