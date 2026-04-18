import { useState } from "react";
import { Button } from "@/components/ui/Button";
import type { Shot, Storyboard } from "@/lib/storyboardCommands";
import { ShotCard } from "./ShotCard";

export interface StoryboardEditorProps {
  storyboard: Storyboard | null;
  onChange: (storyboard: Storyboard) => void;
}

export function StoryboardEditor({ storyboard, onChange }: StoryboardEditorProps) {
  const [dragIndex, setDragIndex] = useState<number | null>(null);

  if (!storyboard) {
    return (
      <div className="flex h-full items-center justify-center font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
        No storyboard yet — generate one above
      </div>
    );
  }

  function updateShot(i: number, patch: Partial<Shot>) {
    if (!storyboard) return;
    const shots = storyboard.shots.map((s, idx) => (idx === i ? { ...s, ...patch } : s));
    onChange({ ...storyboard, shots });
  }

  function removeShot(i: number) {
    if (!storyboard) return;
    const shots = storyboard.shots
      .filter((_, idx) => idx !== i)
      .map((s, idx) => ({ ...s, index: idx + 1 }));
    onChange({ ...storyboard, shots });
  }

  function addShot() {
    if (!storyboard) return;
    const next: Shot = {
      index: storyboard.shots.length + 1,
      description: "",
      style: "",
      duration_s: 4,
      camera: "static",
      transition: "cut",
    };
    onChange({ ...storyboard, shots: [...storyboard.shots, next] });
  }

  function dropOn(i: number) {
    if (dragIndex === null || dragIndex === i) {
      setDragIndex(null);
      return;
    }
    if (!storyboard) return;
    const shots = [...storyboard.shots];
    const [moved] = shots.splice(dragIndex, 1);
    shots.splice(i, 0, moved);
    const renumbered = shots.map((s, idx) => ({ ...s, index: idx + 1 }));
    onChange({ ...storyboard, shots: renumbered });
    setDragIndex(null);
  }

  return (
    <div className="flex h-full flex-col gap-2 overflow-y-auto p-3">
      <div className="flex items-center justify-between">
        <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
          Shots · {storyboard.shots.length}
        </span>
        <Button variant="secondary" size="sm" onClick={addShot}>
          Add shot
        </Button>
      </div>
      {storyboard.shots.map((shot, i) => (
        <ShotCard
          key={shot.index}
          shot={shot}
          onChange={(patch) => updateShot(i, patch)}
          onRemove={() => removeShot(i)}
          onDragStart={() => setDragIndex(i)}
          onDragOver={() => {}}
          onDrop={() => dropOn(i)}
          isDragging={dragIndex === i}
        />
      ))}
    </div>
  );
}
