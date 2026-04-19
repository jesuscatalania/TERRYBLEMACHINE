import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { HelpIcon } from "@/components/ui/HelpIcon";

describe("HelpIcon", () => {
  it("renders the trigger glyph", () => {
    render(<HelpIcon content="explanation" />);
    expect(screen.getByLabelText(/help/i)).toBeInTheDocument();
  });

  it("shows tooltip content on hover", async () => {
    const user = userEvent.setup();
    render(<HelpIcon content="Drops clusters smaller than NxN px" />);
    await user.hover(screen.getByLabelText(/help/i));
    // Tooltip is shown via framer-motion AnimatePresence after openDelay (200ms by default).
    expect(
      await screen.findByText("Drops clusters smaller than NxN px", {}, { timeout: 1000 }),
    ).toBeInTheDocument();
  });
});
