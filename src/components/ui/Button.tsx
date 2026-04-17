import type { ButtonHTMLAttributes, ReactNode, Ref } from "react";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger" | "icon";
export type ButtonSize = "sm" | "md" | "lg";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Visual emphasis. Defaults to `secondary`. */
  variant?: ButtonVariant;
  /** Button height+padding tier. Defaults to `md`. */
  size?: ButtonSize;
  children?: ReactNode;
  /**
   * React 19 forwards refs as a regular prop; we accept and spread it onto
   * the underlying `<button>`. Used by RecentsMenu to focus-return after
   * Escape / click-outside / item selection (FU #95).
   */
  ref?: Ref<HTMLButtonElement>;
}

const BASE =
  "inline-flex items-center justify-center gap-2 rounded-xs border font-mono uppercase tracking-label transition-colors disabled:cursor-not-allowed disabled:opacity-40 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-400";

const SIZE: Record<ButtonSize, string> = {
  sm: "h-6 px-2 text-[10px]",
  md: "h-7 px-3 text-2xs",
  lg: "h-9 px-4 text-[11px]",
};

const ICON_SIZE: Record<ButtonSize, string> = {
  sm: "h-6 w-6 p-0",
  md: "h-7 w-7 p-0",
  lg: "h-9 w-9 p-0",
};

const VARIANT: Record<ButtonVariant, string> = {
  primary: "bg-accent-500 border-accent-500 text-white hover:bg-accent-600 hover:border-accent-600",
  secondary:
    "bg-transparent border-neutral-dark-600 text-neutral-dark-300 hover:text-neutral-dark-50 hover:border-neutral-dark-500",
  ghost: "bg-transparent border-transparent text-neutral-dark-400 hover:text-neutral-dark-100",
  danger: "bg-rose-600 border-rose-600 text-white hover:bg-rose-700 hover:border-rose-700",
  icon: "bg-transparent border-neutral-dark-600 text-neutral-dark-400 hover:text-neutral-dark-100 hover:border-neutral-dark-500",
};

export function Button({
  variant = "secondary",
  size = "md",
  className = "",
  type = "button",
  children,
  ...rest
}: ButtonProps) {
  const sizeClasses = variant === "icon" ? ICON_SIZE[size] : SIZE[size];
  return (
    <button
      type={type}
      data-variant={variant}
      data-size={size}
      className={`${BASE} ${sizeClasses} ${VARIANT[variant]} ${className}`}
      {...rest}
    >
      {children}
    </button>
  );
}
