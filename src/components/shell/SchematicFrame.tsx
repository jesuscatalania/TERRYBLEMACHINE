import type { ReactNode } from "react";

export interface SchematicFrameProps {
  children: ReactNode;
  figLabel?: string;
  tag?: string;
  className?: string;
}

export function SchematicFrame({ children, figLabel, tag, className = "" }: SchematicFrameProps) {
  return (
    <div className={`relative border border-neutral-dark-500/70 px-8 pt-7 pb-8 ${className}`}>
      <span
        data-bracket="tl"
        aria-hidden="true"
        className="-top-[5px] -left-[5px] absolute h-2.5 w-2.5 border-accent-500 border-t border-l"
      />
      <span
        data-bracket="tr"
        aria-hidden="true"
        className="-top-[5px] -right-[5px] absolute h-2.5 w-2.5 border-accent-500 border-t border-r"
      />
      <span
        data-bracket="bl"
        aria-hidden="true"
        className="-bottom-[5px] -left-[5px] absolute h-2.5 w-2.5 border-accent-500 border-b border-l"
      />
      <span
        data-bracket="br"
        aria-hidden="true"
        className="-right-[5px] -bottom-[5px] absolute h-2.5 w-2.5 border-accent-500 border-r border-b"
      />

      {figLabel ? (
        <span className="-top-[7px] absolute left-4 bg-neutral-dark-900 px-2 font-mono text-2xs text-neutral-dark-400 uppercase tracking-label-wide">
          {figLabel}
        </span>
      ) : null}

      {tag ? (
        <span className="-top-[7px] absolute right-4 bg-neutral-dark-900 px-2 font-mono text-2xs text-accent-500 uppercase tracking-label-wide">
          {tag}
        </span>
      ) : null}

      {children}
    </div>
  );
}
