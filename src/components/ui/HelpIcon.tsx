import { HelpCircle } from "lucide-react";
import type { ReactNode } from "react";
import { Tooltip, type TooltipSide } from "@/components/ui/Tooltip";

export interface HelpIconProps {
  content: ReactNode;
  side?: TooltipSide;
}

/**
 * A `?` glyph that shows a Tooltip on hover/focus. Pair with technical-
 * parameter labels (kerning, filter_speckle, etc.) to give just-in-time
 * help without cluttering the UI.
 */
export function HelpIcon({ content, side = "top" }: HelpIconProps) {
  return (
    <Tooltip content={content} side={side}>
      <button
        type="button"
        aria-label="Help"
        className="inline-flex h-3 w-3 items-center justify-center rounded-full text-neutral-dark-500 hover:text-neutral-dark-200"
      >
        <HelpCircle className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
      </button>
    </Tooltip>
  );
}
