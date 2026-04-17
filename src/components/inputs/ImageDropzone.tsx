import { FileImage, ImageUp, X } from "lucide-react";
import { type DragEvent, useEffect, useId, useRef, useState } from "react";

export const ACCEPTED_MIME_TYPES = [
  "image/png",
  "image/jpeg",
  "image/webp",
  "image/svg+xml",
  "image/tiff",
  "image/vnd.adobe.photoshop",
] as const;

export const MAX_FILE_BYTES = 50 * 1024 * 1024; // 50 MB

/** Mime types we can display as inline <img>. TIFF / PSD fall back to a filename chip. */
const RENDERABLE_MIMES = new Set(["image/png", "image/jpeg", "image/webp", "image/svg+xml"]);

export interface ImageDropzoneProps {
  /** Called with the selected File or `null` when cleared. */
  onChange: (file: File | null) => void;
  className?: string;
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

export function ImageDropzone({ onChange, className = "" }: ImageDropzoneProps) {
  const inputId = useId();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [file, setFile] = useState<File | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isDragging, setIsDragging] = useState(false);

  // Keep an object URL alive for as long as we have a renderable file.
  useEffect(() => {
    if (!file || !RENDERABLE_MIMES.has(file.type)) {
      setPreviewUrl(null);
      return;
    }
    const url = URL.createObjectURL(file);
    setPreviewUrl(url);
    return () => URL.revokeObjectURL(url);
  }, [file]);

  const accept = (f: File) => {
    if (!(ACCEPTED_MIME_TYPES as readonly string[]).includes(f.type)) {
      setError(`Unsupported type: ${f.type || "unknown"}`);
      return;
    }
    if (f.size > MAX_FILE_BYTES) {
      setError(`File too large (${formatBytes(f.size)}) — max is ${formatBytes(MAX_FILE_BYTES)}`);
      return;
    }
    setError(null);
    setFile(f);
    onChange(f);
  };

  const handleFileInput = (e: React.ChangeEvent<HTMLInputElement>) => {
    const f = e.target.files?.[0];
    if (f) accept(f);
    // Allow re-selecting the same file later.
    e.target.value = "";
  };

  const handleDragOver = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
  };

  const handleDragLeave = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
  };

  const handleDrop = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
    const f = e.dataTransfer.files?.[0];
    if (f) accept(f);
  };

  const clear = () => {
    setFile(null);
    setError(null);
    onChange(null);
    if (fileInputRef.current) fileInputRef.current.value = "";
  };

  return (
    <div className={`flex flex-col gap-2 ${className}`}>
      <label htmlFor={inputId} className="sr-only">
        Upload image
      </label>
      <input
        ref={fileInputRef}
        id={inputId}
        type="file"
        accept={ACCEPTED_MIME_TYPES.join(",")}
        className="sr-only"
        onChange={handleFileInput}
      />

      {file ? (
        <div className="flex items-center gap-3 rounded-xs border border-neutral-dark-600 bg-neutral-dark-800 p-3">
          {previewUrl ? (
            <img
              src={previewUrl}
              alt={file.name}
              className="h-16 w-16 rounded-xs border border-neutral-dark-700 object-cover"
            />
          ) : (
            <div className="grid h-16 w-16 place-items-center rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 text-neutral-dark-400">
              <FileImage className="h-6 w-6" strokeWidth={1.5} aria-hidden="true" />
            </div>
          )}
          <div className="min-w-0 flex-1">
            <div className="truncate text-neutral-dark-50 text-sm">{file.name}</div>
            <div className="font-mono text-2xs text-neutral-dark-400 tracking-label uppercase">
              {file.type || "unknown"} · {formatBytes(file.size)}
            </div>
          </div>
          <button
            type="button"
            aria-label="Remove"
            onClick={clear}
            className="grid h-6 w-6 place-items-center rounded-xs border border-neutral-dark-600 text-neutral-dark-400 hover:border-neutral-dark-500 hover:text-neutral-dark-100"
          >
            <X className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          </button>
        </div>
      ) : (
        // biome-ignore lint/a11y/noStaticElementInteractions: this is a drop zone; keyboard path is the "browse" button inside it
        <div
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
          className={`flex flex-col items-center justify-center gap-2 rounded-xs border border-dashed px-6 py-8 text-center transition-colors ${
            isDragging
              ? "border-accent-500 bg-accent-500/5"
              : "border-neutral-dark-600 bg-neutral-dark-800/40"
          }`}
        >
          <ImageUp aria-hidden="true" strokeWidth={1.5} className="h-6 w-6 text-neutral-dark-400" />
          <p className="text-neutral-dark-200 text-sm">
            Drop an image here, or{" "}
            <button
              type="button"
              onClick={() => fileInputRef.current?.click()}
              className="font-mono text-accent-500 uppercase tracking-label hover:text-accent-400"
            >
              browse
            </button>
          </p>
          <p className="font-mono text-2xs text-neutral-dark-500 tracking-label uppercase">
            PNG · JPG · WEBP · SVG · TIFF · PSD · MAX {formatBytes(MAX_FILE_BYTES)}
          </p>
        </div>
      )}

      {error ? (
        <div
          role="alert"
          className="rounded-xs border border-rose-500/40 bg-rose-500/10 px-3 py-2 font-mono text-2xs text-rose-300"
        >
          {error}
        </div>
      ) : null}
    </div>
  );
}
