import { AnimatePresence, motion } from "framer-motion";
import { ArrowUp, History, Trash2 } from "lucide-react";
import { type KeyboardEvent, useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/Button";
import { Textarea } from "@/components/ui/Input";
import { usePromptHistoryStore } from "@/stores/promptHistoryStore";

export interface PromptInputProps {
  placeholder?: string;
  /** Controlled value; falls back to internal state when omitted. */
  value?: string;
  /** Controlled setter. */
  onValueChange?: (value: string) => void;
  /** Called with trimmed text on submit (Cmd+Enter or Submit button). */
  onSubmit: (text: string) => void;
  /** Extra className for the outer container. */
  className?: string;
  /** Max height of the textarea before scrolling. */
  maxHeight?: number;
}

export function PromptInput({
  placeholder = "Describe what to build…",
  value,
  onValueChange,
  onSubmit,
  className = "",
  maxHeight = 240,
}: PromptInputProps) {
  const [local, setLocal] = useState("");
  const text = value ?? local;
  const setText = (next: string) => {
    if (onValueChange) onValueChange(next);
    else setLocal(next);
  };

  const entries = usePromptHistoryStore((s) => s.entries);
  const pushHistory = usePromptHistoryStore((s) => s.push);
  const clearHistory = usePromptHistoryStore((s) => s.clear);

  const [historyOpen, setHistoryOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!historyOpen) return;
    function onDocMouseDown(e: MouseEvent) {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setHistoryOpen(false);
      }
    }
    document.addEventListener("mousedown", onDocMouseDown);
    return () => document.removeEventListener("mousedown", onDocMouseDown);
  }, [historyOpen]);

  const submit = () => {
    const trimmed = text.trim();
    if (!trimmed) return;
    pushHistory(trimmed);
    onSubmit(trimmed);
    setText("");
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      submit();
    }
  };

  return (
    <div ref={rootRef} className={`relative flex flex-col gap-2 ${className}`}>
      <Textarea
        value={text}
        onValueChange={setText}
        onKeyDown={onKeyDown}
        placeholder={placeholder}
        rows={3}
        maxHeight={maxHeight}
      />

      <div className="flex items-center justify-between">
        <Button
          variant="secondary"
          size="sm"
          aria-label="History"
          onClick={() => setHistoryOpen((v) => !v)}
          disabled={entries.length === 0}
        >
          <History className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          History
        </Button>

        <div className="flex items-center gap-2">
          <span className="font-mono text-2xs text-neutral-dark-500 tracking-label uppercase">
            ⌘↵ Submit
          </span>
          <Button
            variant="primary"
            size="sm"
            aria-label="Submit"
            onClick={submit}
            disabled={!text.trim()}
          >
            <ArrowUp className="h-3 w-3" strokeWidth={2} aria-hidden="true" />
            Submit
          </Button>
        </div>
      </div>

      <AnimatePresence>
        {historyOpen && entries.length > 0 ? (
          <motion.div
            initial={{ opacity: 0, y: -4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -4 }}
            transition={{ duration: 0.12 }}
            className="absolute top-full left-0 z-30 mt-1 w-full max-w-xl rounded-xs border border-neutral-dark-600 bg-neutral-dark-800 shadow-elevated"
          >
            <div className="flex items-center justify-between border-neutral-dark-700 border-b px-3 py-2">
              <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
                Recent prompts · {entries.length}
              </span>
              <button
                type="button"
                onClick={() => {
                  clearHistory();
                  setHistoryOpen(false);
                }}
                className="flex items-center gap-1 font-mono text-2xs text-neutral-dark-400 hover:text-rose-400"
              >
                <Trash2 className="h-3 w-3" strokeWidth={1.5} />
                CLEAR
              </button>
            </div>
            <ul className="max-h-64 overflow-y-auto">
              {entries.map((entry) => (
                <li key={entry.id}>
                  <button
                    type="button"
                    onClick={() => {
                      setText(entry.text);
                      setHistoryOpen(false);
                    }}
                    className="block w-full truncate px-3 py-2 text-left text-neutral-dark-200 text-sm hover:bg-neutral-dark-700/70"
                  >
                    {entry.text}
                  </button>
                </li>
              ))}
            </ul>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </div>
  );
}
