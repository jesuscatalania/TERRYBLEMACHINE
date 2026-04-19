import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { LoadingButton } from "@/components/ui/LoadingButton";

describe("LoadingButton", () => {
  it("shows label when not loading", () => {
    render(<LoadingButton>Generate</LoadingButton>);
    expect(screen.getByRole("button", { name: /Generate/i })).toBeInTheDocument();
  });

  it("disables button when loading", () => {
    render(<LoadingButton loading>Generate</LoadingButton>);
    expect(screen.getByRole("button")).toBeDisabled();
  });

  it("renders a spinner indicator when loading", () => {
    render(<LoadingButton loading>Generate</LoadingButton>);
    expect(screen.getByTestId("loading-spinner")).toBeInTheDocument();
  });

  it("does not invoke onClick while loading", async () => {
    const user = userEvent.setup();
    const onClick = vi.fn();
    render(
      <LoadingButton loading onClick={onClick}>
        Generate
      </LoadingButton>,
    );
    // disabled buttons don't fire click in userEvent
    await user.click(screen.getByRole("button")).catch(() => {});
    expect(onClick).not.toHaveBeenCalled();
  });
});
