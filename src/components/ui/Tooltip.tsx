import { AnimatePresence, motion } from "framer-motion";
import { Children, cloneElement, type ReactElement, type ReactNode, useRef, useState } from "react";

export type TooltipSide = "top" | "bottom" | "left" | "right";

export interface TooltipProps {
  content: ReactNode;
  children: ReactElement<Record<string, unknown>>;
  /** Positioning relative to the trigger. Defaults to `top`. */
  side?: TooltipSide;
  /** ms before showing. Defaults to 200. */
  openDelay?: number;
  /** ms before hiding after unhover. Defaults to 80. */
  closeDelay?: number;
}

const POSITION: Record<TooltipSide, string> = {
  top: "-top-1 left-1/2 -translate-x-1/2 -translate-y-full",
  bottom: "top-full left-1/2 -translate-x-1/2 translate-y-1",
  left: "-left-1 top-1/2 -translate-y-1/2 -translate-x-full",
  right: "left-full top-1/2 -translate-y-1/2 translate-x-1",
};

export function Tooltip({
  content,
  children,
  side = "top",
  openDelay = 200,
  closeDelay = 80,
}: TooltipProps) {
  const [open, setOpen] = useState(false);
  const openTimer = useRef<number | null>(null);
  const closeTimer = useRef<number | null>(null);

  const clearTimers = () => {
    if (openTimer.current !== null) window.clearTimeout(openTimer.current);
    if (closeTimer.current !== null) window.clearTimeout(closeTimer.current);
    openTimer.current = null;
    closeTimer.current = null;
  };

  const show = () => {
    clearTimers();
    openTimer.current = window.setTimeout(() => setOpen(true), openDelay);
  };

  const hide = () => {
    clearTimers();
    closeTimer.current = window.setTimeout(() => setOpen(false), closeDelay);
  };

  const trigger = Children.only(children) as ReactElement<Record<string, unknown>>;
  const cloned = cloneElement(trigger, {
    onMouseEnter: show,
    onMouseLeave: hide,
    onFocus: show,
    onBlur: hide,
  });

  return (
    <span className="relative inline-flex">
      {cloned}
      <AnimatePresence>
        {open ? (
          <motion.span
            role="tooltip"
            initial={{ opacity: 0, y: side === "top" ? 4 : -4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.12, ease: "easeOut" }}
            className={`pointer-events-none absolute z-40 whitespace-nowrap rounded-xs border border-neutral-dark-600 bg-neutral-dark-900 px-2 py-1 font-mono text-2xs text-neutral-dark-100 uppercase tracking-label shadow-elevated ${POSITION[side]}`}
          >
            {content}
          </motion.span>
        ) : null}
      </AnimatePresence>
    </span>
  );
}
