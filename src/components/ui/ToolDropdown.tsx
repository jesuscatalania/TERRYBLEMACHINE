import { ChevronDown } from "lucide-react";
import { useState } from "react";
import { getToolsFor, type TaskKind, type Tier, type ToolDef } from "@/lib/toolCatalog";

export interface ToolDropdownProps {
  taskKind: TaskKind;
  /** Either "auto" (let router decide) or a Model enum string from the catalog. */
  value: string;
  onChange: (next: string) => void;
}

const TIER_LABEL: Record<Tier, string> = {
  primary: "Primary",
  fallback: "Fallbacks",
  alternative: "Alternatives",
};

export function ToolDropdown({ taskKind, value, onChange }: ToolDropdownProps) {
  const [open, setOpen] = useState(false);
  const tools = getToolsFor(taskKind);
  const selected = value === "auto" ? null : tools.find((t) => t.model === value);
  const groups: Record<Tier, ToolDef[]> = {
    primary: [],
    fallback: [],
    alternative: [],
  };
  for (const t of tools) {
    groups[t.tier].push(t);
  }

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex items-center gap-1 rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 px-2 py-1 font-mono text-2xs text-neutral-dark-100 uppercase tracking-label hover:border-neutral-dark-600"
      >
        {selected ? selected.label : "Auto"}
        <ChevronDown className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
      </button>
      {open ? (
        <div
          role="listbox"
          className="absolute z-10 mt-1 flex min-w-[200px] flex-col gap-2 rounded-xs border border-neutral-dark-700 bg-neutral-dark-900 p-2 shadow-elevated"
        >
          <button
            type="button"
            onClick={() => {
              onChange("auto");
              setOpen(false);
            }}
            className="rounded-xs px-2 py-1 text-left font-mono text-2xs text-neutral-dark-100 uppercase tracking-label hover:bg-neutral-dark-800"
          >
            Auto (router decides)
          </button>
          {(Object.keys(groups) as Tier[]).map((tier) =>
            groups[tier].length > 0 ? (
              <div key={tier} className="flex flex-col gap-1">
                <span className="font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
                  {TIER_LABEL[tier]}
                </span>
                {groups[tier].map((tool) => (
                  <button
                    key={tool.id}
                    type="button"
                    onClick={() => {
                      onChange(tool.model);
                      setOpen(false);
                    }}
                    className={`rounded-xs px-2 py-1 text-left font-mono text-2xs uppercase tracking-label hover:bg-neutral-dark-800 ${value === tool.model ? "text-accent-500" : "text-neutral-dark-100"}`}
                  >
                    {tool.label}
                  </button>
                ))}
              </div>
            ) : null,
          )}
        </div>
      ) : null}
    </div>
  );
}
