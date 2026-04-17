import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Dropdown } from "@/components/ui/Dropdown";
import { Input } from "@/components/ui/Input";
import { Modal } from "@/components/ui/Modal";

export type ExportFormat = "png" | "jpeg" | "webp" | "svg";

export interface ExportSettings {
  format: ExportFormat;
  /** 0.1 – 1.0 for raster formats with lossy encoding. */
  quality: number;
  /** PNG only: preserve transparency. */
  transparent: boolean;
  /** Custom filename (without extension). */
  filename: string;
}

export interface ExportDialogProps {
  open: boolean;
  onClose: () => void;
  onExport: (settings: ExportSettings) => void;
}

const FORMAT_OPTIONS = [
  { value: "png", label: "PNG", hint: "Lossless, supports transparency" },
  { value: "jpeg", label: "JPEG", hint: "Compressed, no transparency" },
  { value: "webp", label: "WebP", hint: "Modern, compressed" },
  { value: "svg", label: "SVG", hint: "Vector, resolution-independent" },
];

export function ExportDialog({ open, onClose, onExport }: ExportDialogProps) {
  const [format, setFormat] = useState<ExportFormat>("png");
  const [quality, setQuality] = useState(90);
  const [transparent, setTransparent] = useState(true);
  const [filename, setFilename] = useState("untitled");

  const isLossy = format === "jpeg" || format === "webp";
  const supportsTransparency = format === "png";

  function handleExport() {
    onExport({
      format,
      quality: quality / 100,
      transparent,
      filename: filename.trim() || "untitled",
    });
  }

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Export"
      maxWidth={420}
      footer={
        <>
          <Button variant="ghost" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" size="sm" onClick={handleExport}>
            Export
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-1.5">
          <label
            htmlFor="export-format"
            className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label"
          >
            Format
          </label>
          <Dropdown
            id="export-format"
            value={format}
            onChange={(v) => setFormat(v as ExportFormat)}
            options={FORMAT_OPTIONS}
          />
        </div>

        <Input
          label="Filename"
          id="export-filename"
          value={filename}
          onValueChange={setFilename}
          placeholder="untitled"
        />

        {isLossy ? (
          <div className="flex flex-col gap-1.5">
            <label
              htmlFor="export-quality"
              className="flex items-center justify-between font-mono text-2xs text-neutral-dark-400 uppercase tracking-label"
            >
              <span>Quality</span>
              <span className="text-accent-500 tabular-nums">{quality}%</span>
            </label>
            <input
              id="export-quality"
              type="range"
              min={70}
              max={100}
              value={quality}
              onChange={(e) => setQuality(Number(e.currentTarget.value))}
              className="w-full accent-accent-500"
            />
          </div>
        ) : null}

        {supportsTransparency ? (
          <label className="flex items-center gap-2 font-mono text-2xs text-neutral-dark-300 uppercase tracking-label">
            <input
              type="checkbox"
              checked={transparent}
              onChange={(e) => setTransparent(e.currentTarget.checked)}
              className="accent-accent-500"
            />
            Transparent background
          </label>
        ) : null}
      </div>
    </Modal>
  );
}
