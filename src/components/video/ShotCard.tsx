import type { DragEvent } from "react";
import type { Shot } from "@/lib/storyboardCommands";

export interface ShotCardProps {
  shot: Shot;
  onChange: (patch: Partial<Shot>) => void;
  onRemove: () => void;
  onDragStart: () => void;
  onDragOver: (e: DragEvent) => void;
  onDrop: () => void;
  isDragging: boolean;
}

export function ShotCard({
  shot,
  onChange,
  onRemove,
  onDragStart,
  onDragOver,
  onDrop,
  isDragging,
}: ShotCardProps) {
  return (
    <article
      draggable
      aria-label={`Shot ${shot.index}`}
      onDragStart={onDragStart}
      onDragOver={(e) => {
        e.preventDefault();
        onDragOver(e);
      }}
      onDrop={onDrop}
      className={`rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 p-3 ${
        isDragging ? "opacity-50" : ""
      }`}
      data-testid={`shot-card-${shot.id}`}
    >
      <div className="flex items-start justify-between gap-2">
        <span className="font-mono text-2xs text-accent-500 uppercase tracking-label">
          Shot {shot.index}
        </span>
        <button
          type="button"
          onClick={onRemove}
          aria-label={`Remove shot ${shot.index}`}
          className="text-neutral-dark-400 hover:text-neutral-dark-100"
        >
          ×
        </button>
      </div>
      <input
        type="text"
        value={shot.description}
        onChange={(e) => onChange({ description: e.target.value })}
        className="mt-1 w-full rounded-xs border border-neutral-dark-700 bg-neutral-dark-950 px-2 py-1 text-xs text-neutral-dark-100"
        placeholder="Description"
        aria-label={`Shot ${shot.index} description`}
      />
      <div className="mt-1 flex gap-2">
        <select
          value={shot.duration_s <= 7 ? 5 : 10}
          onChange={(e) => onChange({ duration_s: Number(e.target.value) })}
          className="w-16 rounded-xs border border-neutral-dark-700 bg-neutral-dark-950 px-2 py-1 text-xs text-neutral-dark-100"
          aria-label={`Shot ${shot.index} duration`}
        >
          <option value={5}>5</option>
          <option value={10}>10</option>
        </select>
        <span className="font-mono text-2xs text-neutral-dark-500 self-center">sec</span>
        <input
          type="text"
          value={shot.camera}
          onChange={(e) => onChange({ camera: e.target.value })}
          className="flex-1 rounded-xs border border-neutral-dark-700 bg-neutral-dark-950 px-2 py-1 text-xs text-neutral-dark-100"
          placeholder="Camera"
          aria-label={`Shot ${shot.index} camera`}
        />
      </div>
    </article>
  );
}
