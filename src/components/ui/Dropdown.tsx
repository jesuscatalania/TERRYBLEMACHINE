import { Check, ChevronDown, Search } from "lucide-react";
import { useEffect, useId, useMemo, useRef, useState } from "react";

export interface DropdownOption {
  value: string;
  label: string;
  hint?: string;
  disabled?: boolean;
}

export interface DropdownProps {
  options: readonly DropdownOption[];
  value?: string;
  onChange: (value: string) => void;
  /** Trigger label when no value is selected. */
  placeholder?: string;
  /** When true, shows a search input that filters the options. */
  searchable?: boolean;
  /** HTML id of the combobox trigger — use for `<label htmlFor>` association. */
  id?: string;
  className?: string;
}

export function Dropdown({
  options,
  value,
  onChange,
  placeholder = "Select…",
  searchable = false,
  id,
  className = "",
}: DropdownProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const listboxId = useId();
  const rootRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) setQuery("");
    if (open && searchable) searchRef.current?.focus();
  }, [open, searchable]);

  useEffect(() => {
    if (!open) return;
    function onDocMouseDown(e: MouseEvent) {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", onDocMouseDown);
    return () => document.removeEventListener("mousedown", onDocMouseDown);
  }, [open]);

  const filtered = useMemo(() => {
    if (!query) return options;
    const q = query.toLowerCase();
    return options.filter((o) => o.label.toLowerCase().includes(q));
  }, [options, query]);

  const selected = options.find((o) => o.value === value);

  return (
    <div ref={rootRef} className={`relative inline-block ${className}`}>
      <button
        type="button"
        id={id}
        role="combobox"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listboxId}
        onClick={() => setOpen((v) => !v)}
        className="flex min-w-[10rem] items-center justify-between gap-2 rounded-xs border border-neutral-dark-600 bg-neutral-dark-800 px-3 py-1.5 text-left text-sm text-neutral-dark-100 hover:border-neutral-dark-500 focus:outline-none focus-visible:ring-1 focus-visible:ring-accent-400"
      >
        <span className={`flex-1 truncate ${!selected ? "text-neutral-dark-400" : ""}`}>
          {selected ? selected.label : placeholder}
        </span>
        <ChevronDown
          aria-hidden="true"
          strokeWidth={1.5}
          className={`h-3.5 w-3.5 text-neutral-dark-400 transition-transform ${open ? "rotate-180" : ""}`}
        />
      </button>

      {open ? (
        <div className="absolute top-full left-0 z-20 mt-1 min-w-full rounded-xs border border-neutral-dark-600 bg-neutral-dark-800 shadow-elevated">
          {searchable ? (
            <div className="flex items-center gap-2 border-neutral-dark-700 border-b px-3 py-1.5">
              <Search
                aria-hidden="true"
                className="h-3 w-3 text-neutral-dark-500"
                strokeWidth={1.5}
              />
              <input
                ref={searchRef}
                type="search"
                value={query}
                onChange={(e) => setQuery(e.currentTarget.value)}
                className="w-full bg-transparent font-mono text-2xs text-neutral-dark-100 placeholder:text-neutral-dark-500 focus:outline-none"
                placeholder="Search…"
              />
            </div>
          ) : null}

          <ul id={listboxId} role="listbox" className="max-h-56 overflow-y-auto py-1">
            {filtered.map((opt) => {
              const selectedNow = opt.value === value;
              const pick = () => {
                if (opt.disabled) return;
                onChange(opt.value);
                setOpen(false);
              };
              return (
                <li
                  key={opt.value}
                  role="option"
                  aria-selected={selectedNow}
                  aria-disabled={opt.disabled}
                  tabIndex={opt.disabled ? -1 : 0}
                  onClick={pick}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      pick();
                    }
                  }}
                  className={`flex cursor-pointer items-center justify-between gap-2 px-3 py-1.5 text-sm outline-none hover:bg-neutral-dark-700/70 focus-visible:bg-neutral-dark-700/70 aria-disabled:cursor-not-allowed aria-disabled:opacity-40 ${
                    selectedNow ? "text-neutral-dark-50" : "text-neutral-dark-200"
                  }`}
                >
                  <span className="flex flex-col">
                    <span>{opt.label}</span>
                    {opt.hint ? (
                      <span className="font-mono text-2xs text-neutral-dark-500 tracking-label uppercase">
                        {opt.hint}
                      </span>
                    ) : null}
                  </span>
                  {selectedNow ? (
                    <Check className="h-3.5 w-3.5 text-accent-500" strokeWidth={2} />
                  ) : null}
                </li>
              );
            })}
            {filtered.length === 0 ? (
              <li className="px-3 py-2 text-center font-mono text-2xs text-neutral-dark-500 tracking-label uppercase">
                No results
              </li>
            ) : null}
          </ul>
        </div>
      ) : null}
    </div>
  );
}
