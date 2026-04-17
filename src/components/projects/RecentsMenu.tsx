import { Clock } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { useProjectStore } from "@/stores/projectStore";

export function RecentsMenu() {
  const [open, setOpen] = useState(false);
  const recents = useProjectStore((s) => s.recents);
  const openProject = useProjectStore((s) => s.openProject);

  return (
    <div className="relative">
      <Button
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
              {recents.map((project) => (
                <li key={project.id}>
                  <button
                    type="button"
                    role="menuitem"
                    className="w-full px-3 py-2 text-left hover:bg-neutral-dark-800 focus:bg-neutral-dark-800 focus:outline-none"
                    onClick={() => {
                      openProject(project);
                      setOpen(false);
                    }}
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
