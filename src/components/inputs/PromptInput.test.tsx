import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PromptInput } from "@/components/inputs/PromptInput";
import { usePromptHistoryStore } from "@/stores/promptHistoryStore";

describe("PromptInput", () => {
  beforeEach(() => {
    usePromptHistoryStore.setState({ entries: [] });
  });

  it("renders an auto-resizing textarea with placeholder", () => {
    render(<PromptInput placeholder="Describe…" onSubmit={vi.fn()} />);
    expect(screen.getByPlaceholderText("Describe…")).toBeInTheDocument();
  });

  it("fires onSubmit with trimmed text + pushes history on click", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(<PromptInput onSubmit={onSubmit} />);
    await user.type(screen.getByRole("textbox"), "  hello world  ");
    await user.click(screen.getByRole("button", { name: /submit/i }));
    expect(onSubmit).toHaveBeenCalledWith("hello world");
    expect(usePromptHistoryStore.getState().entries[0]?.text).toBe("hello world");
  });

  it("submits on Cmd+Enter", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(<PromptInput onSubmit={onSubmit} />);
    await user.click(screen.getByRole("textbox"));
    await user.keyboard("hi{Meta>}{Enter}{/Meta}");
    expect(onSubmit).toHaveBeenCalledWith("hi");
  });

  it("history button is disabled when there are no entries", () => {
    render(<PromptInput onSubmit={vi.fn()} />);
    expect(screen.getByRole("button", { name: /history/i })).toBeDisabled();
  });

  it("clicking history opens the dropdown and picking an entry fills the textarea", async () => {
    const user = userEvent.setup();
    usePromptHistoryStore.getState().push("previous prompt");
    const onSubmit = vi.fn();
    render(<PromptInput onSubmit={onSubmit} />);
    await user.click(screen.getByRole("button", { name: /history/i }));
    await user.click(await screen.findByText("previous prompt"));
    expect((screen.getByRole("textbox") as HTMLTextAreaElement).value).toBe("previous prompt");
  });

  it("does not submit empty input", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(<PromptInput onSubmit={onSubmit} />);
    await user.click(screen.getByRole("button", { name: /submit/i }));
    expect(onSubmit).not.toHaveBeenCalled();
  });
});
