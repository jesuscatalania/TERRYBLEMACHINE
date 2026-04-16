export type Status = "ok" | "warn" | "error";

export interface StatusDotProps {
  status?: Status;
  label?: string;
  className?: string;
}

const STYLES: Record<Status, string> = {
  ok: "bg-emerald-500 shadow-[0_0_0_2px_rgba(16,185,129,0.18)]",
  warn: "bg-amber-400 shadow-[0_0_0_2px_rgba(245,158,11,0.18)]",
  error: "bg-rose-500 shadow-[0_0_0_2px_rgba(244,63,94,0.18)]",
};

export function StatusDot({ status = "ok", label, className = "" }: StatusDotProps) {
  const a11yProps = label ? { role: "img" as const, "aria-label": label } : {};
  return (
    <span
      data-status={status}
      {...a11yProps}
      className={`inline-block h-1.5 w-1.5 rounded-full ${STYLES[status]} ${className}`}
    />
  );
}
