import { useState } from "react";
import { optimizePrompt } from "@/lib/optimizeCommands";
import type { TaskKind } from "@/lib/toolCatalog";

export interface UseOptimizePromptArgs {
  taskKind: TaskKind;
  value: string;
  setValue: (next: string) => void;
}

export function useOptimizePrompt({ taskKind, value, setValue }: UseOptimizePromptArgs) {
  const [enabled, setEnabled] = useState(false);
  const [busy, setBusy] = useState(false);
  const [originalForUndo, setOriginalForUndo] = useState<string | null>(null);

  /**
   * Run optimize-prompt against the current `value`, or against an
   * explicit `inputOverride` when the caller needs to optimize a
   * different string (e.g. a slug-stripped `cleanPrompt`). Resolves to
   * the optimized string on success, or `undefined` if a call was
   * rejected because an optimize run is already in flight.
   *
   * Returning the optimized text (in addition to firing `setValue`) lets
   * callers feed the result straight into the next step of an async
   * pipeline without waiting for React state to commit — useful for the
   * common "optimize then submit" flow where reading `value` in the same
   * tick would still observe the pre-optimize string.
   *
   * Undo always records the *full* raw `value` (slug included), not the
   * `inputOverride`, so the user can restore exactly what they typed.
   * Callers that need to re-attach a slug to the textarea may issue a
   * second `setValue` after this resolves — React batches the writes
   * and the caller's last write wins with no visible flicker.
   */
  async function optimize(inputOverride?: string): Promise<string | undefined> {
    if (busy) return undefined;
    const base = inputOverride ?? value;
    setBusy(true);
    try {
      const optimized = await optimizePrompt(base, taskKind);
      setOriginalForUndo(value);
      setValue(optimized);
      return optimized;
    } finally {
      setBusy(false);
    }
  }

  function undo(): void {
    if (originalForUndo === null) return;
    setValue(originalForUndo);
    setOriginalForUndo(null);
  }

  return {
    enabled,
    setEnabled,
    busy,
    optimize,
    undo,
    canUndo: originalForUndo !== null,
  };
}
