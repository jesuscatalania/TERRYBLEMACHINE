import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { WelcomeModal } from "@/components/onboarding/WelcomeModal";
import { WELCOME_LOCALSTORAGE_KEY } from "@/hooks/useWelcomeFlow";

describe("WelcomeModal", () => {
  afterEach(() => {
    window.localStorage.removeItem(WELCOME_LOCALSTORAGE_KEY);
  });

  it("does not render when localStorage flag is set", () => {
    window.localStorage.setItem(WELCOME_LOCALSTORAGE_KEY, "true");
    render(<WelcomeModal />);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("renders step 1 by default when flag missing", () => {
    window.localStorage.removeItem(WELCOME_LOCALSTORAGE_KEY);
    render(<WelcomeModal />);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText(/Welcome to TERRYBLEMACHINE/i)).toBeInTheDocument();
  });

  it("Next advances through steps; Back returns; Skip + Done dismiss", async () => {
    const user = userEvent.setup();
    window.localStorage.removeItem(WELCOME_LOCALSTORAGE_KEY);
    render(<WelcomeModal />);
    await user.click(screen.getByRole("button", { name: /Next/i }));
    expect(screen.getAllByText(/meingeschmack/i).length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: /Next/i }));
    expect(screen.getByText(/create a project/i)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /^Back$/i }));
    expect(screen.getAllByText(/meingeschmack/i).length).toBeGreaterThan(0);

    // Done on the last step dismisses
    await user.click(screen.getByRole("button", { name: /Next/i })); // back to step 3
    await user.click(screen.getByRole("button", { name: /Done/i }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(window.localStorage.getItem(WELCOME_LOCALSTORAGE_KEY)).toBe("true");
  });

  it("Skip dismisses without completing", async () => {
    const user = userEvent.setup();
    window.localStorage.removeItem(WELCOME_LOCALSTORAGE_KEY);
    render(<WelcomeModal />);
    await user.click(screen.getByRole("button", { name: /Skip/i }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(window.localStorage.getItem(WELCOME_LOCALSTORAGE_KEY)).toBe("true");
  });
});
