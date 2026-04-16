import type { ReactNode } from "react";

export interface KbdProps {
  children: ReactNode;
  className?: string;
}

export function Kbd({ children, className = "" }: KbdProps) {
  return (
    <kbd
      className={`inline-flex items-center rounded-xs border border-neutral-dark-600 px-1.5 py-px font-mono text-2xs text-neutral-dark-500 tabular-nums ${className}`}
    >
      {children}
    </kbd>
  );
}
