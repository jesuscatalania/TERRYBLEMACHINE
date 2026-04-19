import { useEffect } from "react";
import type { ShortcutEntry } from "@/stores/keyboardStore";
import { useKeyboardStore } from "@/stores/keyboardStore";

/**
 * Register a keyboard shortcut for the lifetime of the component.
 *
 * Caller MUST memoize `handler` (useCallback) if it captures state — otherwise
 * the registry re-registers every render. The hook only re-syncs when `id`,
 * `combo`, or `scope` change.
 */
export function useKeyboardShortcut(entry: ShortcutEntry): void {
  const register = useKeyboardStore((s) => s.register);
  const unregister = useKeyboardStore((s) => s.unregister);

  useEffect(() => {
    register(entry);
    return () => unregister(entry.id);
  }, [entry, register, unregister]);
}
