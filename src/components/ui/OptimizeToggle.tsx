import { Loader2, Sparkles, Undo2 } from "lucide-react";

export interface OptimizeToggleProps {
  enabled: boolean;
  onToggle: (next: boolean) => void;
  busy: boolean;
  canUndo: boolean;
  onUndo: () => void;
}

export function OptimizeToggle({ enabled, onToggle, busy, canUndo, onUndo }: OptimizeToggleProps) {
  return (
    <div className="flex items-center gap-2">
      <button
        type="button"
        role="switch"
        aria-checked={enabled}
        aria-busy={busy}
        disabled={busy}
        onClick={() => onToggle(!enabled)}
        className={`flex items-center gap-1 rounded-xs border px-2 py-1 font-mono text-2xs uppercase tracking-label transition-colors ${
          enabled
            ? "border-accent-500 bg-accent-500/10 text-accent-500"
            : "border-neutral-dark-700 bg-neutral-dark-900 text-neutral-dark-400 hover:text-neutral-dark-200"
        }`}
        aria-label="Optimize"
      >
        {busy ? (
          <Loader2 className="h-3 w-3 animate-spin" aria-hidden={true} />
        ) : (
          <Sparkles className="h-3 w-3" strokeWidth={1.5} />
        )}
        Optimize
      </button>
      {canUndo ? (
        <button
          type="button"
          onClick={onUndo}
          className="flex items-center gap-1 rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 font-mono text-2xs text-neutral-dark-400 uppercase tracking-label hover:text-neutral-dark-200"
        >
          <Undo2 className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          Undo
        </button>
      ) : null}
    </div>
  );
}
