import { ArrowUp, Image as ImageIcon, X } from "lucide-react";
import { type KeyboardEvent, useState } from "react";
import { ImageDropzone } from "@/components/inputs/ImageDropzone";
import { Button } from "@/components/ui/Button";
import { Textarea } from "@/components/ui/Input";
import { usePromptHistoryStore } from "@/stores/promptHistoryStore";

export interface CombinedInputSubmission {
  text: string;
  image: File | null;
}

export interface CombinedInputProps {
  placeholder?: string;
  onSubmit: (submission: CombinedInputSubmission) => void;
  className?: string;
  maxHeight?: number;
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

export function CombinedInput({
  placeholder = "Describe what to build…",
  onSubmit,
  className = "",
  maxHeight = 240,
}: CombinedInputProps) {
  const [text, setText] = useState("");
  const [image, setImage] = useState<File | null>(null);
  const [dropzoneOpen, setDropzoneOpen] = useState(false);
  const pushHistory = usePromptHistoryStore((s) => s.push);

  const submit = () => {
    const trimmed = text.trim();
    if (!trimmed && !image) return;
    if (trimmed) pushHistory(trimmed);
    onSubmit({ text: trimmed, image });
    setText("");
    setImage(null);
    setDropzoneOpen(false);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      submit();
    }
  };

  return (
    <div className={`flex flex-col gap-2 ${className}`}>
      <Textarea
        value={text}
        onValueChange={setText}
        onKeyDown={onKeyDown}
        placeholder={placeholder}
        rows={3}
        maxHeight={maxHeight}
      />

      {image ? (
        <div className="flex items-center gap-2 self-start rounded-xs border border-neutral-dark-600 bg-neutral-dark-800 px-2 py-1">
          <ImageIcon
            className="h-3 w-3 text-neutral-dark-400"
            strokeWidth={1.5}
            aria-hidden="true"
          />
          <span className="text-neutral-dark-100 text-sm">{image.name}</span>
          <span className="font-mono text-2xs text-neutral-dark-400 tracking-label uppercase">
            {formatBytes(image.size)}
          </span>
          <button
            type="button"
            aria-label="Remove attachment"
            onClick={() => setImage(null)}
            className="ml-1 grid h-4 w-4 place-items-center rounded-xs text-neutral-dark-400 hover:text-neutral-dark-100"
          >
            <X className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          </button>
        </div>
      ) : null}

      {dropzoneOpen && !image ? (
        <ImageDropzone
          onChange={(f) => {
            setImage(f);
            if (f) setDropzoneOpen(false);
          }}
        />
      ) : null}

      <div className="flex items-center justify-between">
        <Button
          variant="secondary"
          size="sm"
          aria-label="Attach"
          onClick={() => setDropzoneOpen((v) => !v)}
          disabled={!!image}
        >
          <ImageIcon className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          Attach
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
            disabled={!text.trim() && !image}
          >
            <ArrowUp className="h-3 w-3" strokeWidth={2} aria-hidden="true" />
            Submit
          </Button>
        </div>
      </div>
    </div>
  );
}
