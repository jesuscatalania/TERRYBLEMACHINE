import { Modal } from "@/components/ui/Modal";
import type { ShortcutScope } from "@/stores/keyboardStore";
import { useKeyboardStore } from "@/stores/keyboardStore";

const SCOPE_LABEL: Record<ShortcutScope, string> = {
  global: "Global",
  page: "This page",
  "module:website": "Website Builder",
  "module:graphic2d": "2D Graphic",
  "module:graphic3d": "Pseudo-3D",
  "module:video": "Video",
  "module:typography": "Typography",
};

export interface ShortcutHelpOverlayProps {
  open: boolean;
  onClose: () => void;
}

export function ShortcutHelpOverlay({ open, onClose }: ShortcutHelpOverlayProps) {
  // NOTE: subscribe to the raw `entries` Map (stable identity until register/
  // unregister mutates it) and convert inside render — calling `s.list()`
  // inside the selector returns a fresh array every read and triggers
  // Zustand's "snapshot changed" loop.
  const entriesMap = useKeyboardStore((s) => s.entries);
  const entries = Array.from(entriesMap.values());

  const groups = new Map<ShortcutScope, typeof entries>();
  for (const e of entries) {
    const arr = groups.get(e.scope) ?? [];
    arr.push(e);
    groups.set(e.scope, arr);
  }

  return (
    <Modal open={open} onClose={onClose} title="Keyboard shortcuts" maxWidth={520}>
      <div className="flex flex-col gap-4">
        {Array.from(groups.entries()).map(([scope, list]) => (
          <div key={scope} className="flex flex-col gap-2">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              {SCOPE_LABEL[scope]}
            </span>
            <ul className="flex flex-col gap-1">
              {list.map((e) => (
                <li
                  key={e.id}
                  className="flex items-center justify-between text-2xs text-neutral-dark-200"
                >
                  <span>{e.label}</span>
                  <kbd className="rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-1.5 py-0.5 font-mono text-2xs text-neutral-dark-300">
                    {e.combo}
                  </kbd>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </Modal>
  );
}
