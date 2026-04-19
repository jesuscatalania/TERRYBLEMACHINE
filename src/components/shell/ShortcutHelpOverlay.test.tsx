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

  it("orders scope groups by priority (page → module → global) regardless of registration order", () => {
    // Register in reverse priority order — global first, then module,
    // then page. After sorting, the rendered DOM order should be the
    // opposite: page → module → global.
    useKeyboardStore.getState().register({
      id: "g:cmdN",
      combo: "Mod+N",
      handler: () => {},
      scope: "global",
      label: "New project",
    });
    useKeyboardStore.getState().register({
      id: "m:typo:exportKit",
      combo: "Mod+E",
      handler: () => {},
      scope: "module:typography",
      label: "Export brand kit",
    });
    useKeyboardStore.getState().register({
      id: "p:focusPrompt",
      combo: "Mod+L",
      handler: () => {},
      scope: "page",
      label: "Focus prompt",
    });
    render(<ShortcutHelpOverlay open={true} onClose={() => {}} />);
    const headings = screen.getAllByText(/^(Global|This page|Typography)$/);
    expect(headings.map((n) => n.textContent)).toEqual(["This page", "Typography", "Global"]);
  });
});
