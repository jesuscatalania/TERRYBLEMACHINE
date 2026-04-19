import { Loader2 } from "lucide-react";
import { Button, type ButtonProps } from "@/components/ui/Button";

export interface LoadingButtonProps extends ButtonProps {
  /** When true, the button is disabled and shows a spinner next to its label. */
  loading?: boolean;
}

/**
 * Button that renders a leading spinner and becomes disabled while `loading`.
 * Use instead of ad-hoc `busy ? "X…" : "X"` label toggles so loading
 * affordances are consistent across modules.
 */
export function LoadingButton({ loading, disabled, children, ...rest }: LoadingButtonProps) {
  return (
    <Button {...rest} disabled={disabled || loading} aria-busy={loading || undefined}>
      {loading ? (
        <Loader2
          data-testid="loading-spinner"
          className="h-3 w-3 animate-spin"
          strokeWidth={1.5}
          aria-hidden="true"
        />
      ) : null}
      {children}
    </Button>
  );
}
