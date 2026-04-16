import { getModule } from "@/components/shell/modules";
import { SchematicFrame } from "@/components/shell/SchematicFrame";
import { Badge } from "@/components/ui/Badge";
import type { ModuleId } from "@/stores/appStore";

export interface ModulePlaceholderProps {
  moduleId: ModuleId;
}

export function ModulePlaceholder({ moduleId }: ModulePlaceholderProps) {
  const mod = getModule(moduleId);
  return (
    <div className="flex h-full items-center justify-center p-10">
      <SchematicFrame
        figLabel={`FIG ${mod.index} — PENDING`}
        tag={mod.tag}
        className="w-full max-w-xl"
      >
        <div className="flex flex-col items-center gap-5 py-4 text-center">
          <Badge tone="warn">Coming soon</Badge>
          <h1 className="font-display text-3xl font-bold text-neutral-dark-50 tracking-tight">
            Coming soon — {mod.label}
          </h1>
          <p className="max-w-sm text-neutral-dark-300 leading-relaxed">
            Module <span className="font-mono text-accent-500">{mod.tag}</span> is scheduled for a
            later phase. Check{" "}
            <span className="font-mono text-neutral-dark-200">docs/ENTWICKLUNGSPLAN.md</span> for
            the roadmap.
          </p>
        </div>
      </SchematicFrame>
    </div>
  );
}
