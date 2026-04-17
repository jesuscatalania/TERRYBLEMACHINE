import { Clock } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/Button";
import { useProjectStore } from "@/stores/projectStore";

export function RecentsMenu() {
  const [open, setOpen] = useState(false);
  const [focusIndex, setFocusIndex] = useState(0);
  const recents = useProjectStore((s) => s.recents);
  const openProject = useProjectStore((s) => s.openProject);

  // Container ref — used for the click-outside detection.
  const containerRef = useRef<HTMLDivElement>(null);
  // Trigger ref — focus returns here on close.
  const triggerRef = useRef<HTMLButtonElement>(null);
  // Refs for each menu item so ArrowDown/Up/Home/End can move focus
  // without relying on DOM queries.
  const itemRefs = useRef<Array<HTMLButtonElement | null>>([]);

  // Close helper that always restores focus to the trigger. Callers pass
  // `true` when the close originated from an interaction that should
  // audibly/visually return focus (Escape, click-outside, item selection).
  const closeAndRestore = useCallback(() => {
    setOpen(false);
    // Defer focus until the portal/menu is gone — otherwise React re-entry
    // during the same synchronous tick can steal focus back.
    queueMicrotask(() => {
      triggerRef.current?.focus();
    });
  }, []);

  // Reset the roving tabindex when the menu opens.
  useEffect(() => {
    if (open) {
      setFocusIndex(0);
    }
  }, [open]);

  // Click-outside dismissal.
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        closeAndRestore();
      }
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [open, closeAndRestore]);

  // Global Escape handler while open.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        closeAndRestore();
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, closeAndRestore]);

  // When focusIndex changes while the menu is open, push focus to the
  // corresponding item (the roving part of roving-tabindex).
  useEffect(() => {
    if (!open) return;
    const el = itemRefs.current[focusIndex];
    el?.focus();
  }, [open, focusIndex]);

  const onItemKeyDown = (e: React.KeyboardEvent<HTMLButtonElement>) => {
    const last = recents.length - 1;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setFocusIndex((i) => (i >= last ? 0 : i + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setFocusIndex((i) => (i <= 0 ? last : i - 1));
    } else if (e.key === "Home") {
      e.preventDefault();
      setFocusIndex(0);
    } else if (e.key === "End") {
      e.preventDefault();
      setFocusIndex(last);
    }
  };

  return (
    <div ref={containerRef} className="relative">
      <Button
        ref={triggerRef}
        variant="ghost"
        size="sm"
        aria-label="Recent projects"
        aria-expanded={open}
        aria-haspopup="menu"
        onClick={() => setOpen((v) => !v)}
      >
        <Clock className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
        Recent
      </Button>
      {open ? (
        <div
          role="menu"
          className="absolute right-0 top-full z-20 mt-1 w-64 rounded-xs border border-neutral-dark-600 bg-neutral-dark-900 shadow-xl"
        >
          {recents.length === 0 ? (
            <div className="p-3 font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              No recent projects
            </div>
          ) : (
            <ul className="max-h-80 overflow-y-auto py-1">
              {recents.map((project, idx) => (
                <li key={project.id}>
                  <button
                    ref={(el) => {
                      itemRefs.current[idx] = el;
                    }}
                    type="button"
                    role="menuitem"
                    // Roving tabindex: only the currently-focused item is
                    // reachable with Tab, the rest use -1.
                    tabIndex={idx === focusIndex ? 0 : -1}
                    className="w-full px-3 py-2 text-left hover:bg-neutral-dark-800 focus:bg-neutral-dark-800 focus:outline-none"
                    onClick={() => {
                      openProject(project);
                      closeAndRestore();
                    }}
                    onKeyDown={onItemKeyDown}
                  >
                    <div className="text-xs text-neutral-dark-100">{project.name}</div>
                    <div className="font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
                      {project.module}
                    </div>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : null}
    </div>
  );
}
