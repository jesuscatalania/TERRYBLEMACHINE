import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

// Monaco pulls in Web Workers + language services that don't initialise in
// jsdom. Stub the component to a plain textarea so we can exercise the
// CodeEditor's props and UI chrome without loading the real editor.
vi.mock("@monaco-editor/react", () => ({
  default: (props: {
    value?: string;
    onChange?: (v: string) => void;
    language?: string;
    path?: string;
  }) => (
    <textarea
      data-testid="monaco-stub"
      data-language={props.language}
      data-path={props.path}
      defaultValue={props.value ?? ""}
      onChange={(e) => props.onChange?.(e.currentTarget.value)}
    />
  ),
}));

import { CodeEditor } from "@/components/website/CodeEditor";
import type { GeneratedFile } from "@/lib/websiteCommands";

const htmlFile: GeneratedFile = { path: "index.html", content: "<h1>Hi</h1>" };

describe("CodeEditor", () => {
  it("renders the mock editor with the active file's content", () => {
    render(<CodeEditor files={[htmlFile]} onChange={() => {}} />);
    const ta = screen.getByTestId("monaco-stub") as HTMLTextAreaElement;
    expect(ta.value).toBe("<h1>Hi</h1>");
  });

  it("forwards the derived Monaco language for the active file", () => {
    render(<CodeEditor files={[htmlFile]} onChange={() => {}} />);
    const ta = screen.getByTestId("monaco-stub") as HTMLTextAreaElement;
    expect(ta.getAttribute("data-language")).toBe("html");
  });

  it("propagates edits through onChange with the updated file list", () => {
    const onChange = vi.fn();
    render(<CodeEditor files={[htmlFile]} onChange={onChange} />);
    fireEvent.change(screen.getByTestId("monaco-stub"), {
      target: { value: "<h1>new</h1>" },
    });
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith([{ path: "index.html", content: "<h1>new</h1>" }]);
  });

  it("shows the Modify button only when onRequestAssist is provided", () => {
    const { rerender } = render(<CodeEditor files={[htmlFile]} onChange={() => {}} />);
    expect(screen.queryByRole("button", { name: /modify/i })).toBeNull();

    rerender(
      <CodeEditor files={[htmlFile]} onChange={() => {}} onRequestAssist={async () => ""} />,
    );
    expect(screen.getByRole("button", { name: /modify/i })).toBeInTheDocument();
  });

  it("renders an empty-state message when no files are provided", () => {
    render(<CodeEditor files={[]} onChange={() => {}} />);
    expect(screen.queryByTestId("monaco-stub")).toBeNull();
    expect(screen.getByText(/no files to edit/i)).toBeInTheDocument();
  });
});
