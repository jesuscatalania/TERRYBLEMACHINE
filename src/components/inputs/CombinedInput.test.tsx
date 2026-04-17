import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CombinedInput } from "@/components/inputs/CombinedInput";
import { usePromptHistoryStore } from "@/stores/promptHistoryStore";

function makeImage(name = "pic.png") {
  return new File([new Uint8Array(10)], name, { type: "image/png" });
}

describe("CombinedInput", () => {
  beforeEach(() => {
    usePromptHistoryStore.setState({ entries: [] });
  });

  it("renders the prompt textarea and an Attach button", () => {
    render(<CombinedInput onSubmit={vi.fn()} />);
    expect(screen.getByRole("textbox")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /attach/i })).toBeInTheDocument();
  });

  it("submits text with no attachment", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(<CombinedInput onSubmit={onSubmit} />);
    await user.type(screen.getByRole("textbox"), "plain prompt");
    await user.click(screen.getByRole("button", { name: /submit/i }));
    expect(onSubmit).toHaveBeenCalledWith({
      text: "plain prompt",
      image: null,
    });
  });

  it("Attach opens the dropzone; picking a file shows an inline chip", async () => {
    const user = userEvent.setup();
    render(<CombinedInput onSubmit={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: /attach/i }));
    const file = makeImage("logo.png");
    await user.upload(screen.getByLabelText(/upload image/i), file);
    // collapsed chip shows filename
    expect(screen.getByText("logo.png")).toBeInTheDocument();
  });

  it("submits text + image when both are set", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(<CombinedInput onSubmit={onSubmit} />);
    await user.type(screen.getByRole("textbox"), "with image");
    await user.click(screen.getByRole("button", { name: /attach/i }));
    const file = makeImage();
    await user.upload(screen.getByLabelText(/upload image/i), file);
    await user.click(screen.getByRole("button", { name: /submit/i }));
    expect(onSubmit).toHaveBeenCalledWith({
      text: "with image",
      image: file,
    });
  });

  it("clears text and image after a successful submit", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(<CombinedInput onSubmit={onSubmit} />);
    await user.type(screen.getByRole("textbox"), "one-shot");
    await user.click(screen.getByRole("button", { name: /submit/i }));
    expect((screen.getByRole("textbox") as HTMLTextAreaElement).value).toBe("");
  });
});
