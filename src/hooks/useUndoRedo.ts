import { useCallback } from "react";
import { useKeyboardShortcut } from "@/hooks/useKeyboardShortcut";
import { useHistoryStore } from "@/stores/historyStore";

/**
 * Mount once (typically in `App`) to wire keyboard shortcuts to the history
 * store. macOS `⌘+Z` and Windows/Linux `Ctrl+Z` undo; adding `Shift` redoes.
 *
 * Shortcuts are registered through the keyboard registry, so the global
 * dispatcher (`useGlobalKeyboardDispatch`) handles text-field suppression and
 * scope-priority resolution.
 */
export function useUndoRedo(): void {
  const undo = useCallback(() => {
    useHistoryStore.getState().undo();
  }, []);
  const redo = useCallback(() => {
    useHistoryStore.getState().redo();
  }, []);

  useKeyboardShortcut({
    id: "global:undo",
    combo: "Mod+Z",
    handler: undo,
    scope: "global",
    label: "Undo",
  });
  useKeyboardShortcut({
    id: "global:redo",
    combo: "Mod+Shift+Z",
    handler: redo,
    scope: "global",
    label: "Redo",
  });
}
