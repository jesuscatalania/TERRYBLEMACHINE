import { StatusDot } from "@/components/shell/StatusDot";
import { useAiStore } from "@/stores/aiStore";

export interface StatusBarProps {
  cacheCapacity?: number;
  renderProgress?: number;
}

function formatUsd(cents: number): string {
  return `$${(cents / 100).toFixed(2)}`;
}

export function StatusBar({ cacheCapacity = 500, renderProgress }: StatusBarProps) {
  const budget = useAiStore((s) => s.budget);
  const cache = useAiStore((s) => s.cache);
  const activeRequests = useAiStore((s) => s.activeRequests);

  const active = activeRequests.length > 0;
  const aiLabel = active ? "AI · ACTIVE" : "AI · IDLE";
  const progressPercent =
    renderProgress !== undefined ? Math.round(renderProgress * 100) : undefined;

  return (
    <footer
      role="contentinfo"
      className="flex h-7 items-center justify-between border-neutral-dark-600 border-t bg-neutral-dark-900 px-3 font-mono text-2xs tracking-label text-neutral-dark-400 uppercase tabular-nums"
    >
      <div className="flex items-center gap-4">
        <span className="flex items-center gap-1.5">
          <StatusDot status={active ? "warn" : "ok"} label={aiLabel} />
          <span>{aiLabel}</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span>CACHE</span>
          <span className="text-neutral-dark-100">
            {cache.size} / {cacheCapacity}
          </span>
        </span>
        <span className="flex items-center gap-1.5">
          <span>BUDGET</span>
          <span className="text-neutral-dark-100">
            {formatUsd(budget.usedCents)} / {formatUsd(budget.limitCents)}
          </span>
        </span>
      </div>

      <div className="flex items-center gap-4">
        <span className="flex items-center gap-1.5">
          <span>QUEUE</span>
          <span className="text-neutral-dark-100">{activeRequests.length}</span>
        </span>
        {progressPercent !== undefined ? (
          <span className="flex items-center gap-2">
            <span>RENDER</span>
            <span
              aria-hidden="true"
              className="relative block h-1 w-20 overflow-hidden rounded-xs bg-neutral-dark-700"
            >
              <span
                className="absolute top-0 left-0 h-full bg-accent-500"
                style={{ width: `${progressPercent}%` }}
              />
            </span>
            <span className="text-accent-500">{progressPercent}%</span>
          </span>
        ) : null}
        <span>⌘K</span>
      </div>
    </footer>
  );
}
