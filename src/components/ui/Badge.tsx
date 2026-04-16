import type { ReactNode } from "react";

export type BadgeTone = "neutral" | "success" | "warn" | "error" | "accent";

export interface BadgeProps {
  children: ReactNode;
  /** Semantic tone. Defaults to `neutral`. */
  tone?: BadgeTone;
  className?: string;
}

const TONES: Record<BadgeTone, string> = {
  neutral: "border-neutral-dark-600 text-neutral-dark-300 bg-neutral-dark-800/60",
  success: "border-emerald-500/40 text-emerald-400 bg-emerald-500/10",
  warn: "border-amber-400/40 text-amber-300 bg-amber-400/10",
  error: "border-rose-500/50 text-rose-400 bg-rose-500/10",
  accent: "border-accent-500/50 text-accent-500 bg-accent-500/10",
};

export function Badge({ children, tone = "neutral", className = "" }: BadgeProps) {
  return (
    <span
      data-tone={tone}
      className={`inline-flex items-center gap-1 rounded-xs border px-1.5 py-px font-mono text-2xs tracking-label uppercase tabular-nums ${TONES[tone]} ${className}`}
    >
      {children}
    </span>
  );
}
