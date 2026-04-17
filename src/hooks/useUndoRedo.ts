import { useEffect } from "react";
import { useHistoryStore } from "@/stores/historyStore";

function isTextField(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA") return true;
  if ((target as HTMLElement).isContentEditable) return true;
  return false;
}

/**
 * Mount once (typically in `App`) to wire keyboard shortcuts to the history
 * store. macOS `⌘+Z` and Windows/Linux `Ctrl+Z` undo; adding `Shift` redoes.
 *
 * Shortcuts are suppressed while focus is in an `<input>`, `<textarea>`, or
 * contenteditable element so native text editing undo still works.
 */
export function useUndoRedo(): void {
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const mod = event.metaKey || event.ctrlKey;
      if (!mod || event.key.toLowerCase() !== "z") return;
      if (isTextField(event.target)) return;

      event.preventDefault();
      const store = useHistoryStore.getState();
      if (event.shiftKey) store.redo();
      else store.undo();
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, []);
}
