import type { ReactNode } from "react";

export interface TabItem {
  id: string;
  label: string;
  icon?: ReactNode;
  disabled?: boolean;
}

export interface TabsProps {
  items: readonly TabItem[];
  activeId: string;
  onChange: (id: string) => void;
  className?: string;
}

export function Tabs({ items, activeId, onChange, className = "" }: TabsProps) {
  return (
    <div
      role="tablist"
      className={`flex items-center gap-1 border-neutral-dark-700 border-b ${className}`}
    >
      {items.map((item) => {
        const active = item.id === activeId;
        const base =
          "flex h-8 items-center gap-2 border-b-2 px-3 font-mono text-2xs uppercase tracking-label transition-colors disabled:cursor-not-allowed disabled:opacity-40";
        const state = active
          ? "border-accent-500 text-neutral-dark-50"
          : "border-transparent text-neutral-dark-400 hover:text-neutral-dark-100";
        return (
          <button
            key={item.id}
            type="button"
            role="tab"
            aria-selected={active}
            aria-controls={`tab-panel-${item.id}`}
            disabled={item.disabled}
            onClick={() => {
              if (!item.disabled) onChange(item.id);
            }}
            className={`${base} ${state}`}
          >
            {item.icon ? <span className="inline-flex h-3.5 w-3.5">{item.icon}</span> : null}
            <span>{item.label}</span>
          </button>
        );
      })}
    </div>
  );
}
