import { render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useGlobalKeyboardDispatch } from "@/hooks/useGlobalKeyboardDispatch";
import { useKeyboardShortcut } from "@/hooks/useKeyboardShortcut";
import { useKeyboardStore } from "@/stores/keyboardStore";

function Probe({
  id,
  combo,
  handler,
  scope,
}: {
  id: string;
  combo: string;
  handler: () => void;
  scope: "global" | "page";
}) {
  useKeyboardShortcut({ id, combo, handler, scope, label: id });
  return null;
}

function DispatcherProbe() {
  useGlobalKeyboardDispatch();
  return null;
}

describe("useKeyboardShortcut + dispatcher", () => {
  afterEach(() => {
    useKeyboardStore.setState({ entries: new Map() });
  });

  it("registers on mount, unregisters on unmount", () => {
    const { unmount } = render(<Probe id="x" combo="Mod+S" handler={() => {}} scope="global" />);
    expect(useKeyboardStore.getState().list()).toHaveLength(1);
    unmount();
    expect(useKeyboardStore.getState().list()).toHaveLength(0);
  });

  it("dispatcher fires the matching handler", () => {
    const handler = vi.fn();
    render(
      <>
        <DispatcherProbe />
        <Probe id="x" combo="Mod+S" handler={handler} scope="global" />
      </>,
    );
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "s", metaKey: true }));
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it("page scope wins over global for the same combo", () => {
    const global = vi.fn();
    const page = vi.fn();
    render(
      <>
        <DispatcherProbe />
        <Probe id="g" combo="Mod+Enter" handler={global} scope="global" />
        <Probe id="p" combo="Mod+Enter" handler={page} scope="page" />
      </>,
    );
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", metaKey: true }));
    expect(page).toHaveBeenCalledTimes(1);
    expect(global).not.toHaveBeenCalled();
  });

  it("text-field focus suppresses dispatch", () => {
    const handler = vi.fn();
    render(
      <>
        <DispatcherProbe />
        <Probe id="x" combo="Mod+Z" handler={handler} scope="global" />
        <textarea data-testid="ta" />
      </>,
    );
    const ta = document.querySelector('[data-testid="ta"]') as HTMLTextAreaElement;
    ta.focus();
    ta.dispatchEvent(new KeyboardEvent("keydown", { key: "z", metaKey: true, bubbles: true }));
    expect(handler).not.toHaveBeenCalled();
  });
});
