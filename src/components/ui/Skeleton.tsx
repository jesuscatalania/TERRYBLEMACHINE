import type { CSSProperties } from "react";

export interface SkeletonProps {
  /** CSS width — number is px, string is passed through. */
  width?: number | string;
  /** CSS height — number is px, string is passed through. */
  height?: number | string;
  className?: string;
}

function toCss(v: number | string | undefined): string | undefined {
  if (v === undefined) return undefined;
  return typeof v === "number" ? `${v}px` : v;
}

export function Skeleton({ width, height, className = "" }: SkeletonProps) {
  const style: CSSProperties = {
    width: toCss(width),
    height: toCss(height),
  };
  return (
    <span
      data-skeleton="true"
      aria-hidden="true"
      style={style}
      className={`inline-block animate-pulse rounded-xs bg-neutral-dark-700/70 ${className}`}
    />
  );
}
