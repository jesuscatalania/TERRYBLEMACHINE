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

  async function optimize(): Promise<void> {
    if (busy) return;
    setBusy(true);
    try {
      const optimized = await optimizePrompt(value, taskKind);
      setOriginalForUndo(value);
      setValue(optimized);
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
