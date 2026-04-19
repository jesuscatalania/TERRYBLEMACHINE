import { useEffect } from "react";
import { eventToCombo } from "@/lib/canonicalCombo";
import { useKeyboardStore } from "@/stores/keyboardStore";

const SCOPE_PRIORITY: Record<string, number> = {
  page: 0,
  "module:website": 1,
  "module:graphic2d": 1,
  "module:graphic3d": 1,
  "module:video": 1,
  "module:typography": 1,
  global: 2,
};

function isTextField(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA") return true;
  if (target.isContentEditable) return true;
  return false;
}

/**
 * Mount once at the App root. Listens to document keydown events,
 * canonicalizes them, finds the highest-priority registered handler, and
 * dispatches. Suppresses dispatch in text-fields (so native text editing
 * undo / cursor movement still works).
 */
export function useGlobalKeyboardDispatch(): void {
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (isTextField(e.target)) return;
      const combo = eventToCombo(e);
      const matches = useKeyboardStore.getState().entriesByCombo(combo);
      if (matches.length === 0) return;
      const passing = matches.filter((m) => !m.when || m.when());
      if (passing.length === 0) return;
      passing.sort((a, b) => (SCOPE_PRIORITY[a.scope] ?? 99) - (SCOPE_PRIORITY[b.scope] ?? 99));
      const winner = passing[0];
      if (!winner) return;
      e.preventDefault();
      winner.handler();
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, []);
}
