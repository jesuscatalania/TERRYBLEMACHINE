import { render, screen, waitForElementToBeRemoved } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Tooltip } from "@/components/ui/Tooltip";

describe("Tooltip", () => {
  it("hides tooltip by default", () => {
    render(
      <Tooltip content="Hello">
        <button type="button">Target</button>
      </Tooltip>,
    );
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  it("shows tooltip on hover", async () => {
    const user = userEvent.setup();
    render(
      <Tooltip content="Hello" openDelay={0}>
        <button type="button">Target</button>
      </Tooltip>,
    );
    await user.hover(screen.getByRole("button", { name: "Target" }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Hello");
  });

  it("hides tooltip on unhover", async () => {
    const user = userEvent.setup();
    render(
      <Tooltip content="Hello" openDelay={0} closeDelay={0}>
        <button type="button">Target</button>
      </Tooltip>,
    );
    const trigger = screen.getByRole("button", { name: "Target" });
    await user.hover(trigger);
    await screen.findByRole("tooltip");
    await user.unhover(trigger);
    await waitForElementToBeRemoved(() => screen.queryByRole("tooltip"));
    expect(screen.queryByRole("tooltip")).toBeNull();
  });
});
