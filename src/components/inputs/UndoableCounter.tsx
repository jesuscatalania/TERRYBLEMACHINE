import { Minus, Plus, Redo2, Undo2 } from "lucide-react";
import { useState } from "react";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { useHistoryStore } from "@/stores/historyStore";

/**
 * Demo widget showing the Command-Pattern pipeline: every click pushes a
 * commend onto the shared `useHistoryStore`. ⌘Z / ⌘⇧Z (wired globally via
 * `useUndoRedo`) or the inline buttons revert / replay the change.
 */
export function UndoableCounter() {
  const [value, setValue] = useState(0);
  const past = useHistoryStore((s) => s.past);
  const future = useHistoryStore((s) => s.future);
  const push = useHistoryStore((s) => s.push);
  const undo = useHistoryStore((s) => s.undo);
  const redo = useHistoryStore((s) => s.redo);

  const bump = (delta: number) => {
    let before = 0;
    push({
      label: delta > 0 ? "Increment counter" : "Decrement counter",
      do: () => {
        setValue((v) => {
          before = v;
          return v + delta;
        });
      },
      undo: () => {
        setValue(before);
      },
    });
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2 font-mono text-2xs text-neutral-dark-400 tracking-label uppercase">
          Value
        </div>
        <div className="font-display font-bold text-3xl text-neutral-dark-50 tabular-nums">
          {value}
        </div>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <Button variant="secondary" size="sm" onClick={() => bump(1)}>
          <Plus className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          Inc
        </Button>
        <Button variant="secondary" size="sm" onClick={() => bump(-1)}>
          <Minus className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          Dec
        </Button>
        <span className="mx-2 h-4 w-px bg-neutral-dark-700" aria-hidden="true" />
        <Button
          variant="ghost"
          size="sm"
          onClick={() => undo()}
          disabled={past.length === 0}
          aria-label="Undo"
        >
          <Undo2 className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          Undo
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => redo()}
          disabled={future.length === 0}
          aria-label="Redo"
        >
          <Redo2 className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          Redo
        </Button>
      </div>

      <div className="flex items-center gap-2">
        <Badge tone={past.length > 0 ? "accent" : "neutral"}>Past · {past.length}</Badge>
        <Badge tone={future.length > 0 ? "accent" : "neutral"}>Future · {future.length}</Badge>
        <span className="font-mono text-2xs text-neutral-dark-500 tracking-label uppercase">
          ⌘Z undo · ⌘⇧Z redo
        </span>
      </div>
    </div>
  );
}
