import { ChevronLeft, ChevronRight, Layers } from "lucide-react";
import { useNavigate } from "react-router-dom";
import { MODULES } from "@/components/shell/modules";
import { SidebarItem } from "@/components/shell/SidebarItem";
import { moduleToPath } from "@/lib/moduleRoutes";
import type { ModuleId } from "@/stores/appStore";
import { useAppStore } from "@/stores/appStore";
import { useProjectStore } from "@/stores/projectStore";

function BrandMark() {
  return (
    <span aria-hidden="true" className="relative inline-block h-5 w-5 border border-accent-500">
      <span className="-translate-x-1/2 absolute top-0 left-1/2 h-full w-px bg-accent-500" />
      <span className="-translate-y-1/2 absolute top-1/2 left-0 h-px w-full bg-accent-500" />
    </span>
  );
}

function SectionLabel({ children }: { children: string }) {
  return (
    <div className="px-4 pt-4 pb-2 font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
      {children}
    </div>
  );
}

export function Sidebar() {
  const { activeModule, setActiveModule, sidebarOpen, toggleSidebar } = useAppStore();
  const currentProject = useProjectStore((s) => s.currentProject);
  const navigate = useNavigate();

  const selectModule = (id: ModuleId) => {
    setActiveModule(id);
    navigate(moduleToPath(id));
  };

  return (
    <aside className="flex h-full flex-col border-neutral-dark-600 border-r bg-neutral-dark-900">
      {/* Brand */}
      <div
        className={`flex h-12 items-center border-neutral-dark-600 border-b ${
          sidebarOpen ? "gap-2.5 px-4" : "justify-center px-0"
        }`}
      >
        <BrandMark />
        {sidebarOpen && (
          <span className="font-mono font-semibold text-[11.5px] text-neutral-dark-100 uppercase tracking-label-wide">
            TERRYBLEMACHINE
          </span>
        )}
      </div>

      {/* Modules */}
      {sidebarOpen && <SectionLabel>Modules</SectionLabel>}
      <nav aria-label="Modules" className={`flex flex-col ${sidebarOpen ? "" : "pt-2"}`}>
        {MODULES.map((m) => (
          <SidebarItem
            key={m.id}
            moduleId={m.id}
            label={m.label}
            index={m.index}
            shortcut={m.shortcut}
            active={m.id === activeModule}
            collapsed={!sidebarOpen}
            onSelect={selectModule}
          />
        ))}
      </nav>

      {/* Project */}
      {sidebarOpen && (
        <>
          <SectionLabel>Project</SectionLabel>
          <div className="grid h-9 grid-cols-[28px_1fr_auto] items-center gap-2.5 border-l-2 border-l-transparent px-4 text-[13px] text-neutral-dark-300">
            <Layers className="h-3.5 w-3.5 justify-self-center" strokeWidth={1.5} />
            <span className="font-medium">
              {currentProject?.name ?? "Untitled"}{" "}
              <span className="ml-1 font-mono text-2xs text-neutral-dark-500">·</span>
            </span>
            <span className="inline-flex items-center rounded-xs border border-neutral-dark-600 px-1.5 py-px font-mono text-2xs text-neutral-dark-500">
              ⌘O
            </span>
          </div>
        </>
      )}

      {/* Bottom bar */}
      <div
        className={`mt-auto flex items-center border-neutral-dark-600 border-t py-3 ${
          sidebarOpen ? "justify-between px-4" : "justify-center px-0"
        }`}
      >
        {sidebarOpen && <span className="font-mono text-2xs text-neutral-dark-400">v0.1.0</span>}
        <button
          type="button"
          onClick={toggleSidebar}
          aria-label={sidebarOpen ? "Collapse sidebar" : "Expand sidebar"}
          className="grid h-5 w-5 place-items-center rounded-xs border border-neutral-dark-600 text-neutral-dark-400 hover:border-neutral-dark-500 hover:text-neutral-dark-100"
        >
          {sidebarOpen ? (
            <ChevronLeft className="h-3 w-3" strokeWidth={1.6} />
          ) : (
            <ChevronRight className="h-3 w-3" strokeWidth={1.6} />
          )}
        </button>
      </div>
    </aside>
  );
}
