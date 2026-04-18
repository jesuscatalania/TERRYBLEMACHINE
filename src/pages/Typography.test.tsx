import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

vi.mock("@/lib/logoCommands", () => ({
  generateLogoVariants: vi.fn(),
}));

import { TypographyPage } from "@/pages/Typography";

describe("TypographyPage", () => {
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
});
