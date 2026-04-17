import type { ReactNode } from "react";
import { Header } from "@/components/shell/Header";
import { Sidebar } from "@/components/shell/Sidebar";
import { StatusBar } from "@/components/shell/StatusBar";
import { useAppStore } from "@/stores/appStore";
import { useProjectStore } from "@/stores/projectStore";

export interface ShellProps {
  children?: ReactNode;
  onNew?: () => void;
  onGenerate?: () => void;
  onOpenSettings?: () => void;
  renderProgress?: number;
}

export function Shell({ children, onNew, onGenerate, onOpenSettings, renderProgress }: ShellProps) {
  const currentProject = useProjectStore((s) => s.currentProject);
  const sidebarOpen = useAppStore((s) => s.sidebarOpen);
  const gridCols = sidebarOpen ? "grid-cols-[15rem_1fr]" : "grid-cols-[3.5rem_1fr]";

  return (
    <div
      data-testid="shell-grid"
      className={`grid h-screen w-screen ${gridCols} grid-rows-[3rem_1fr_1.75rem] bg-neutral-dark-900 text-neutral-dark-100`}
    >
      <div className="row-span-3 row-start-1">
        <Sidebar />
      </div>
      <Header
        projectName={currentProject?.name}
        onNew={onNew}
        onGenerate={onGenerate}
        onOpenSettings={onOpenSettings}
      />
      <main className="relative overflow-hidden">
        {/* Schematic grid background */}
        <span
          aria-hidden="true"
          className="pointer-events-none absolute inset-0 opacity-20"
          style={{
            backgroundImage:
              "linear-gradient(#2a2a30 1px, transparent 1px), linear-gradient(90deg, #2a2a30 1px, transparent 1px)",
            backgroundSize: "40px 40px",
            maskImage: "radial-gradient(circle at 50% 40%, black 0%, transparent 70%)",
            WebkitMaskImage: "radial-gradient(circle at 50% 40%, black 0%, transparent 70%)",
          }}
        />
        <div className="relative h-full overflow-auto">{children}</div>
      </main>
      <StatusBar renderProgress={renderProgress} />
    </div>
  );
}
