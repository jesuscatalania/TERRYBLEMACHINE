import { Settings } from "lucide-react";
import { Breadcrumbs } from "@/components/shell/Breadcrumbs";
import { Button } from "@/components/shell/Button";
import { getModule } from "@/components/shell/modules";
import { useAppStore } from "@/stores/appStore";

export interface HeaderProps {
  projectName?: string;
  onNew?: () => void;
  onGenerate?: () => void;
  onOpenSettings?: () => void;
}

export function Header({
  projectName = "Untitled",
  onNew,
  onGenerate,
  onOpenSettings,
}: HeaderProps) {
  const activeModule = useAppStore((s) => s.activeModule);
  const mod = getModule(activeModule);
  const parts = ["TM", mod.label.toUpperCase(), projectName.toUpperCase()];

  return (
    <header className="flex h-12 items-center justify-between border-neutral-dark-600 border-b bg-neutral-dark-900 px-5">
      <Breadcrumbs parts={parts} />

      <div className="flex items-center gap-2.5">
        <Button onClick={onNew}>New</Button>
        <Button variant="primary" onClick={onGenerate}>
          Generate
        </Button>
        <Button variant="icon" aria-label="Settings" onClick={onOpenSettings}>
          <Settings className="h-3.5 w-3.5" strokeWidth={1.5} aria-hidden="true" />
        </Button>
      </div>
    </header>
  );
}
