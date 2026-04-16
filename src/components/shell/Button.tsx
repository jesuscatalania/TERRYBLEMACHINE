import type { ButtonHTMLAttributes, ReactNode } from "react";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "icon";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  children?: ReactNode;
}

const BASE =
  "inline-flex items-center justify-center gap-2 rounded-xs border font-mono text-2xs uppercase tracking-label transition-colors disabled:cursor-not-allowed disabled:opacity-40 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-400";

const VARIANTS: Record<ButtonVariant, string> = {
  primary:
    "h-7 px-3 bg-accent-500 border-accent-500 text-white hover:bg-accent-600 hover:border-accent-600",
  secondary:
    "h-7 px-3 bg-transparent border-neutral-dark-600 text-neutral-dark-300 hover:text-neutral-dark-50 hover:border-neutral-dark-500",
  ghost:
    "h-7 px-3 bg-transparent border-transparent text-neutral-dark-400 hover:text-neutral-dark-100",
  icon: "h-7 w-7 p-0 bg-transparent border-neutral-dark-600 text-neutral-dark-400 hover:text-neutral-dark-100 hover:border-neutral-dark-500",
};

export function Button({
  variant = "secondary",
  className = "",
  type = "button",
  children,
  ...rest
}: ButtonProps) {
  return (
    <button
      type={type}
      data-variant={variant}
      className={`${BASE} ${VARIANTS[variant]} ${className}`}
      {...rest}
    >
      {children}
    </button>
  );
}
