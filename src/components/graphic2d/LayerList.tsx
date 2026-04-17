import { Eye, EyeOff, Lock, Trash2, Unlock } from "lucide-react";
import type { FabricLayer } from "@/components/graphic2d/FabricCanvas";

export interface LayerListProps {
  layers: readonly FabricLayer[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onToggleVisible: (id: string) => void;
  onToggleLock: (id: string) => void;
  onRemove: (id: string) => void;
}

export function LayerList({
  layers,
  selectedId,
  onSelect,
  onToggleVisible,
  onToggleLock,
  onRemove,
}: LayerListProps) {
  if (layers.length === 0) {
    return (
      <div className="flex h-full items-center justify-center p-4 text-center text-2xs text-neutral-dark-500 font-mono uppercase tracking-label">
        Add an image or text to get started.
      </div>
    );
  }
  return (
    <ul className="flex flex-col divide-y divide-neutral-dark-700">
      {[...layers].reverse().map((layer) => {
        const selected = layer.id === selectedId;
        return (
          <li
            key={layer.id}
            className={`flex items-center gap-2 px-3 py-2 ${
              selected ? "bg-neutral-dark-800/70" : "hover:bg-neutral-dark-800/40"
            }`}
          >
            <button
              type="button"
              onClick={() => onToggleVisible(layer.id)}
              aria-label={layer.visible ? "Hide" : "Show"}
              className="text-neutral-dark-400 hover:text-neutral-dark-100"
            >
              {layer.visible ? (
                <Eye className="h-3 w-3" strokeWidth={1.5} />
              ) : (
                <EyeOff className="h-3 w-3" strokeWidth={1.5} />
              )}
            </button>
            <button
              type="button"
              onClick={() => onToggleLock(layer.id)}
              aria-label={layer.locked ? "Unlock" : "Lock"}
              className="text-neutral-dark-400 hover:text-neutral-dark-100"
            >
              {layer.locked ? (
                <Lock className="h-3 w-3" strokeWidth={1.5} />
              ) : (
                <Unlock className="h-3 w-3" strokeWidth={1.5} />
              )}
            </button>
            <button
              type="button"
              onClick={() => onSelect(layer.id)}
              className={`flex-1 truncate text-left text-sm ${
                selected ? "text-neutral-dark-50" : "text-neutral-dark-200"
              }`}
            >
              {layer.label}
            </button>
            <button
              type="button"
              onClick={() => onRemove(layer.id)}
              aria-label="Remove"
              className="text-neutral-dark-400 hover:text-rose-400"
            >
              <Trash2 className="h-3 w-3" strokeWidth={1.5} />
            </button>
          </li>
        );
      })}
    </ul>
  );
}
