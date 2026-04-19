import { useEffect, useRef } from "react";
import type { ShortcutEntry } from "@/stores/keyboardStore";
import { useKeyboardStore } from "@/stores/keyboardStore";

/**
 * Register a keyboard shortcut for the lifetime of the component.
 *
 * The hook re-syncs ONLY when the primitive identity fields (`id`, `combo`,
 * `scope`) change. The latest `handler` and `when` are read through a ref so
 * callers don't need to memoize them — closures over fresh state are picked
 * up automatically without re-registering on every render.
 */
export function useKeyboardShortcut(entry: ShortcutEntry): void {
  const register = useKeyboardStore((s) => s.register);
  const unregister = useKeyboardStore((s) => s.unregister);

  const { id, combo, scope, label } = entry;

  // Latest handler + when stored in refs so the registered ShortcutEntry
  // always invokes the current closure, even though we don't re-register
  // when those references change. Without this, a non-memoized handler
  // would re-register every render (per-paint churn + a brief
  // unregister/register window during which the shortcut is unbound).
  const handlerRef = useRef(entry.handler);
  const whenRef = useRef(entry.when);
  handlerRef.current = entry.handler;
  whenRef.current = entry.when;

  useEffect(() => {
    register({
      id,
      combo,
      scope,
      label,
      handler: () => handlerRef.current(),
      when: whenRef.current ? () => whenRef.current?.() ?? false : undefined,
    });
    return () => unregister(id);
  }, [id, combo, scope, label, register, unregister]);
}
