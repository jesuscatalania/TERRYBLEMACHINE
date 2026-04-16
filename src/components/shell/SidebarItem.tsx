import { Kbd } from "@/components/shell/Kbd";
import { ModuleIcon } from "@/components/shell/ModuleIcon";
import type { ModuleId } from "@/stores/appStore";

export interface SidebarItemProps {
  moduleId: ModuleId;
  label: string;
  index: string;
  shortcut: string;
  active: boolean;
  onSelect: (id: ModuleId) => void;
}

export function SidebarItem({
  moduleId,
  label,
  index,
  shortcut,
  active,
  onSelect,
}: SidebarItemProps) {
  const base =
    "group grid h-9 w-full cursor-pointer grid-cols-[28px_1fr_auto] items-center gap-2.5 border-l-2 px-4 text-left text-[13px] transition-colors focus:outline-none focus-visible:ring-1 focus-visible:ring-accent-400";
  const state = active
    ? "bg-neutral-dark-800 border-l-accent-500 text-neutral-dark-50"
    : "border-l-transparent text-neutral-dark-300 hover:bg-neutral-dark-800/60 hover:text-neutral-dark-50";

  return (
    <button
      type="button"
      aria-current={active ? "page" : undefined}
      className={`${base} ${state}`}
      onClick={() => onSelect(moduleId)}
    >
      <ModuleIcon moduleId={moduleId} className="justify-self-center" />
      <span className="font-medium">
        {label}{" "}
        <span
          className={`ml-1 font-mono text-2xs ${active ? "text-accent-500" : "text-neutral-dark-500"}`}
        >
          / {index}
        </span>
      </span>
      <Kbd>{shortcut}</Kbd>
    </button>
  );
}
