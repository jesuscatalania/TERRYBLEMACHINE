import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { LoadingButton } from "@/components/ui/LoadingButton";

export interface AssistPopoverProps {
  /** The currently selected code — displayed read-only for context. */
  selection: string;
  /** Fired when the user clicks Apply with a non-empty instruction. */
  onSubmit: (instruction: string) => void | Promise<void>;
  /** Fired when the user clicks Cancel. */
  onClose: () => void;
  /** True while the backend is generating a replacement. */
  busy: boolean;
}

/**
 * Modal popover for Claude-Assist inline edits.
 *
 * Shows the selected snippet + a free-text "Change to…" field. Apply is
 * disabled while the instruction is empty or the backend is busy.
 */
export function AssistPopover({ selection, onSubmit, onClose, busy }: AssistPopoverProps) {
  const [instruction, setInstruction] = useState("");
  const trimmed = instruction.trim();
  const canApply = trimmed.length > 0 && !busy;

  async function handleApply() {
    if (!canApply) return;
    await onSubmit(trimmed);
  }

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Modify code selection"
      className="fixed inset-0 z-30 flex items-center justify-center bg-neutral-dark-950/60"
    >
      <div className="w-[28rem] max-w-[90vw] rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 p-4">
        <div className="mb-2 font-mono text-2xs uppercase tracking-label text-accent-500">
          Modify selection
        </div>
        <pre
          data-testid="assist-selection"
          className="mb-3 max-h-32 overflow-auto rounded-xs bg-neutral-dark-950 p-2 text-2xs text-neutral-dark-300 whitespace-pre-wrap"
        >
          {selection.slice(0, 500)}
        </pre>
        <Input
          label="Change to"
          id="assist-instruction"
          placeholder="Make headline larger and center it"
          value={instruction}
          onValueChange={setInstruction}
          onKeyDown={(e) => {
            if (e.key === "Enter" && canApply) {
              e.preventDefault();
              void handleApply();
            } else if (e.key === "Escape") {
              e.preventDefault();
              onClose();
            }
          }}
        />
        <div className="mt-3 flex justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={onClose} disabled={busy}>
            Cancel
          </Button>
          <LoadingButton
            variant="primary"
            size="sm"
            onClick={handleApply}
            disabled={trimmed.length === 0}
            loading={busy}
          >
            Apply
          </LoadingButton>
        </div>
      </div>
    </div>
  );
}
