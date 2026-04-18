import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Modal } from "@/components/ui/Modal";

export type VideoResolution = "sd" | "hd" | "1080";
export type VideoFormat = "mp4" | "gif";
export type VideoFps = 24 | 30 | 60;

export interface RenderSettings {
  resolution: VideoResolution;
  format: VideoFormat;
  fps: VideoFps;
  filename: string;
}

interface Props {
  open: boolean;
  onClose: () => void;
  onExport: (settings: RenderSettings) => void;
}

export function RenderExportDialog({ open, onClose, onExport }: Props) {
  const [resolution, setResolution] = useState<VideoResolution>("hd");
  const [format, setFormat] = useState<VideoFormat>("mp4");
  const [fps, setFps] = useState<VideoFps>(30);
  const [filename, setFilename] = useState("terryble-video");

  return (
    <Modal open={open} onClose={onClose} title="Export video">
      <div className="flex flex-col gap-3">
        <label className="flex flex-col gap-1">
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            Resolution
          </span>
          <select
            aria-label="Resolution"
            value={resolution}
            onChange={(e) => setResolution(e.target.value as VideoResolution)}
            className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 text-neutral-dark-100 text-xs"
          >
            <option value="sd">720p (SD)</option>
            <option value="hd">1080p (HD)</option>
            <option value="1080">1080p+</option>
          </select>
        </label>

        <label className="flex flex-col gap-1">
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            Format
          </span>
          <select
            aria-label="Format"
            value={format}
            onChange={(e) => setFormat(e.target.value as VideoFormat)}
            className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 text-neutral-dark-100 text-xs"
          >
            <option value="mp4">MP4</option>
            <option value="gif">GIF</option>
          </select>
        </label>

        <label className="flex flex-col gap-1">
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            FPS
          </span>
          <select
            aria-label="FPS"
            value={fps}
            onChange={(e) => setFps(Number(e.target.value) as VideoFps)}
            className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 text-neutral-dark-100 text-xs"
          >
            <option value={24}>24</option>
            <option value={30}>30</option>
            <option value={60}>60</option>
          </select>
        </label>

        <label className="flex flex-col gap-1">
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            Filename
          </span>
          <input
            aria-label="Filename"
            type="text"
            value={filename}
            onChange={(e) => setFilename(e.target.value)}
            className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-950 px-2 py-1 text-neutral-dark-100 text-xs"
          />
        </label>

        <div className="mt-2 flex justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={() => onExport({ resolution, format, fps, filename })}
          >
            Export
          </Button>
        </div>
      </div>
    </Modal>
  );
}
