import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ShortcutHelpOverlay } from "@/components/shell/ShortcutHelpOverlay";
import { useKeyboardStore } from "@/stores/keyboardStore";

describe("ShortcutHelpOverlay", () => {
  afterEach(() => {
    useKeyboardStore.setState({ entries: new Map() });
  });

  it("renders nothing when closed", () => {
    render(<ShortcutHelpOverlay open={false} onClose={() => {}} />);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("lists registered shortcuts grouped by scope when open", () => {
    useKeyboardStore.getState().register({
      id: "g:undo",
      combo: "Mod+Z",
      handler: () => {},
      scope: "global",
      label: "Undo",
    });
    useKeyboardStore.getState().register({
      id: "g:redo",
      combo: "Mod+Shift+Z",
      handler: () => {},
      scope: "global",
      label: "Redo",
    });
    render(<ShortcutHelpOverlay open={true} onClose={() => {}} />);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText("Undo")).toBeInTheDocument();
    expect(screen.getByText("Redo")).toBeInTheDocument();
    expect(screen.getByText("Mod+Z")).toBeInTheDocument();
  });
});
