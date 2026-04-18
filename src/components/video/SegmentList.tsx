import { useState } from "react";
import type { Segment } from "@/stores/videoStore";

export interface SegmentListProps {
  segments: Segment[];
  onDelete: (id: string) => void;
  onReorder: (from: number, to: number) => void;
  onSelect?: (id: string) => void;
  selectedId?: string | null;
}

export function SegmentList({
  segments,
  onDelete,
  onReorder,
  onSelect,
  selectedId,
}: SegmentListProps) {
  const [dragIndex, setDragIndex] = useState<number | null>(null);

  if (segments.length === 0) {
    return (
      <div className="flex h-full items-center justify-center font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
        No segments yet
      </div>
    );
  }

  function dropOn(i: number) {
    if (dragIndex === null || dragIndex === i) {
      setDragIndex(null);
      return;
    }
    onReorder(dragIndex, i);
    setDragIndex(null);
  }

  return (
    <ul className="flex flex-col gap-1 p-2">
      {segments.map((seg, i) => {
        const isSelected = selectedId === seg.id;
        return (
          <li
            key={seg.id}
            draggable
            onDragStart={() => setDragIndex(i)}
            onDragOver={(e) => e.preventDefault()}
            onDrop={() => dropOn(i)}
            className={`flex items-center justify-between gap-2 rounded-xs border px-2 py-1.5 text-xs ${
              isSelected
                ? "border-accent-500 bg-neutral-dark-900"
                : "border-neutral-dark-700 bg-neutral-dark-950"
            } ${dragIndex === i ? "opacity-50" : ""}`}
            data-testid={`segment-${seg.id}`}
          >
            <button
              type="button"
              onClick={() => onSelect?.(seg.id)}
              className="flex flex-1 items-center gap-2 text-left text-neutral-dark-100"
            >
              <span className="font-mono text-2xs text-accent-500 uppercase tracking-label">
                {seg.kind}
              </span>
              <span className="truncate">{seg.label}</span>
              <span className="ml-auto font-mono text-2xs text-neutral-dark-500">
                {seg.duration_s}s
              </span>
              {seg.busy ? (
                <span className="font-mono text-2xs text-neutral-dark-400">…</span>
              ) : null}
              {seg.error ? (
                <span className="font-mono text-2xs text-red-400" title={seg.error}>
                  !
                </span>
              ) : null}
            </button>
            <button
              type="button"
              onClick={() => onDelete(seg.id)}
              aria-label={`Delete segment ${seg.label}`}
              className="text-neutral-dark-400 hover:text-neutral-dark-100"
            >
              ×
            </button>
          </li>
        );
      })}
    </ul>
  );
}
