import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Dropdown } from "@/components/ui/Dropdown";
import { Input } from "@/components/ui/Input";
import { Modal } from "@/components/ui/Modal";

export type ThreeExportFormat = "png" | "jpeg" | "webp" | "pdf";

export interface ThreeExportSettings {
  format: ThreeExportFormat;
  /** 0.1 – 1.0 for raster formats with lossy encoding (jpeg/webp). */
  quality: number;
  /** PNG only: preserve transparency. */
  transparent: boolean;
  /** Custom filename (without extension). */
  filename: string;
}

export interface ThreeExportDialogProps {
  open: boolean;
  onClose: () => void;
  onExport: (settings: ThreeExportSettings) => void;
}

const FORMAT_OPTIONS = [
  { value: "png", label: "PNG", hint: "Lossless, supports transparency" },
  { value: "jpeg", label: "JPEG", hint: "Compressed, no transparency" },
  { value: "webp", label: "WebP", hint: "Modern, compressed" },
  { value: "pdf", label: "PDF", hint: "Single-page document" },
];

export function ThreeExportDialog({ open, onClose, onExport }: ThreeExportDialogProps) {
  const [format, setFormat] = useState<ThreeExportFormat>("png");
  const [quality, setQuality] = useState(92);
  const [transparent, setTransparent] = useState(false);
  const [filename, setFilename] = useState("terryble-3d");

  const isLossy = format === "jpeg" || format === "webp";
  const supportsTransparency = format === "png";

  function handleExport() {
    onExport({
      format,
      quality: quality / 100,
      transparent,
      filename: filename.trim() || "terryble-3d",
    });
  }

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Export 3D scene"
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
            htmlFor="three-export-format"
            className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label"
          >
            Format
          </label>
          <Dropdown
            id="three-export-format"
            value={format}
            onChange={(v) => setFormat(v as ThreeExportFormat)}
            options={FORMAT_OPTIONS}
          />
        </div>

        <Input
          label="Filename"
          id="three-export-filename"
          value={filename}
          onValueChange={setFilename}
          placeholder="terryble-3d"
        />

        {isLossy ? (
          <div className="flex flex-col gap-1.5">
            <label
              htmlFor="three-export-quality"
              className="flex items-center justify-between font-mono text-2xs text-neutral-dark-400 uppercase tracking-label"
            >
              <span>Quality</span>
              <span className="text-accent-500 tabular-nums">{quality}%</span>
            </label>
            <input
              id="three-export-quality"
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
