import type { ReactNode } from "react";

export type CardVariant = "default" | "schematic";

export interface CardProps {
  children: ReactNode;
  /** `default` = flat bordered surface; `schematic` = industrial frame with orange corner brackets. */
  variant?: CardVariant;
  className?: string;
}

function CornerBrackets() {
  return (
    <>
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
    </>
  );
}

export function Card({ children, variant = "default", className = "" }: CardProps) {
  const base =
    variant === "schematic"
      ? "relative border border-neutral-dark-500/70 bg-neutral-dark-900"
      : "rounded-xs border border-neutral-dark-700 bg-neutral-dark-800/40";
  return (
    <div data-variant={variant} className={`${base} ${className}`}>
      {variant === "schematic" ? <CornerBrackets /> : null}
      {children}
    </div>
  );
}

export interface CardHeaderProps {
  children: ReactNode;
  className?: string;
}

export function CardHeader({ children, className = "" }: CardHeaderProps) {
  return (
    <div
      className={`border-neutral-dark-700 border-b px-4 py-3 font-mono text-2xs text-neutral-dark-400 uppercase tracking-label ${className}`}
    >
      {children}
    </div>
  );
}

export interface CardBodyProps {
  children: ReactNode;
  className?: string;
}

export function CardBody({ children, className = "" }: CardBodyProps) {
  return <div className={`px-4 py-4 text-sm text-neutral-dark-100 ${className}`}>{children}</div>;
}

export interface CardFooterProps {
  children: ReactNode;
  className?: string;
}

export function CardFooter({ children, className = "" }: CardFooterProps) {
  return (
    <div
      className={`flex items-center justify-end gap-2 border-neutral-dark-700 border-t px-4 py-2 ${className}`}
    >
      {children}
    </div>
  );
}
