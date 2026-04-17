import { Kbd } from "@/components/shell/Kbd";
import { ModuleIcon } from "@/components/shell/ModuleIcon";
import type { ModuleId } from "@/stores/appStore";

export interface SidebarItemProps {
  moduleId: ModuleId;
  label: string;
  index: string;
  shortcut: string;
  active: boolean;
  collapsed?: boolean;
  onSelect: (id: ModuleId) => void;
}

export function SidebarItem({
  moduleId,
  label,
  index,
  shortcut,
  active,
  collapsed = false,
  onSelect,
}: SidebarItemProps) {
  const baseLayout = collapsed
    ? "grid-cols-[1fr] justify-items-center px-0"
    : "grid-cols-[28px_1fr_auto] gap-2.5 px-4";
  const base = `group grid h-9 w-full cursor-pointer items-center border-l-2 text-left text-[13px] transition-colors focus:outline-none focus-visible:ring-1 focus-visible:ring-accent-400 ${baseLayout}`;
  const state = active
    ? "bg-neutral-dark-800 border-l-accent-500 text-neutral-dark-50"
    : "border-l-transparent text-neutral-dark-300 hover:bg-neutral-dark-800/60 hover:text-neutral-dark-50";

  return (
    <button
      type="button"
      aria-current={active ? "page" : undefined}
      aria-label={collapsed ? label : undefined}
      title={collapsed ? label : undefined}
      className={`${base} ${state}`}
      onClick={() => onSelect(moduleId)}
    >
      <ModuleIcon moduleId={moduleId} className="justify-self-center" />
      {!collapsed && (
        <>
          <span className="font-medium">
            {label}{" "}
            <span
              className={`ml-1 font-mono text-2xs ${active ? "text-accent-500" : "text-neutral-dark-500"}`}
            >
              / {index}
            </span>
          </span>
          <Kbd>{shortcut}</Kbd>
        </>
      )}
    </button>
  );
}
