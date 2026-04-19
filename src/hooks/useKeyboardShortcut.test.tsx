import { render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useGlobalKeyboardDispatch } from "@/hooks/useGlobalKeyboardDispatch";
import { useKeyboardShortcut } from "@/hooks/useKeyboardShortcut";
import { useKeyboardStore } from "@/stores/keyboardStore";
import { useModalStackStore } from "@/stores/modalStackStore";

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
    useModalStackStore.setState({ stack: [] });
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

  it("suppresses non-help shortcuts when a modal is open", () => {
    const nav = vi.fn();
    const newProject = vi.fn();
    render(
      <>
        <DispatcherProbe />
        <Probe id="nav2" combo="Mod+2" handler={nav} scope="global" />
        <Probe id="new" combo="Mod+N" handler={newProject} scope="global" />
      </>,
    );
    // Simulate any modal open.
    useModalStackStore.getState().push("some-modal");
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "2", metaKey: true }));
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "n", metaKey: true }));
    expect(nav).not.toHaveBeenCalled();
    expect(newProject).not.toHaveBeenCalled();
  });

  it("lets help overlay shortcuts (? and Mod+/) through even when a modal is open", () => {
    const helpQ = vi.fn();
    const helpSlash = vi.fn();
    render(
      <>
        <DispatcherProbe />
        <Probe id="help-q" combo="?" handler={helpQ} scope="global" />
        <Probe id="help-slash" combo="Mod+/" handler={helpSlash} scope="global" />
      </>,
    );
    useModalStackStore.getState().push("some-modal");
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "?" }));
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "/", metaKey: true }));
    expect(helpQ).toHaveBeenCalledTimes(1);
    expect(helpSlash).toHaveBeenCalledTimes(1);
  });
});
