import { render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useGlobalKeyboardDispatch } from "@/hooks/useGlobalKeyboardDispatch";
import { useUndoRedo } from "@/hooks/useUndoRedo";
import { useHistoryStore } from "@/stores/historyStore";
import { useKeyboardStore } from "@/stores/keyboardStore";

function Probe() {
  useUndoRedo();
  useGlobalKeyboardDispatch();
  return <div>probe</div>;
}

describe("useUndoRedo", () => {
  beforeEach(() => {
    useHistoryStore.setState({ past: [], future: [] });
    useKeyboardStore.setState({ entries: new Map() });
  });

  it("Cmd+Z calls undo, Cmd+Shift+Z calls redo", async () => {
    const user = userEvent.setup();
    const doFn = vi.fn();
    const undoFn = vi.fn();
    render(<Probe />);

    useHistoryStore.getState().push({ label: "x", do: doFn, undo: undoFn });
    expect(doFn).toHaveBeenCalledOnce();

    await user.keyboard("{Meta>}z{/Meta}");
    expect(undoFn).toHaveBeenCalledOnce();

    doFn.mockClear();
    await user.keyboard("{Meta>}{Shift>}z{/Shift}{/Meta}");
    expect(doFn).toHaveBeenCalledOnce();
  });

  it("ignores Cmd+Z while typing in a text input", async () => {
    const user = userEvent.setup();
    const undoFn = vi.fn();

    function Harness() {
      useUndoRedo();
      useGlobalKeyboardDispatch();
      return <input aria-label="field" defaultValue="" />;
    }
    const { getByLabelText } = render(<Harness />);
    useHistoryStore.getState().push({ label: "x", do: () => {}, undo: undoFn });

    await user.click(getByLabelText("field"));
    await user.keyboard("{Meta>}z{/Meta}");
    expect(undoFn).not.toHaveBeenCalled();
  });

  it("also responds to Ctrl+Z on non-Mac keyboards", async () => {
    const user = userEvent.setup();
    const undoFn = vi.fn();
    render(<Probe />);
    useHistoryStore.getState().push({ label: "x", do: () => {}, undo: undoFn });
    await user.keyboard("{Control>}z{/Control}");
    expect(undoFn).toHaveBeenCalledOnce();
  });
});
