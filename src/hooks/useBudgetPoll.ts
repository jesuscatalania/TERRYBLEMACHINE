import { useEffect } from "react";
import { getBudgetStatus } from "@/lib/budgetCommands";
import { useAiStore } from "@/stores/aiStore";

/**
 * Keeps `useAiStore.budget` in sync with the Rust BudgetManager by polling
 * `get_budget_status` every `intervalMs`. Fails silently if the Tauri runtime
 * is unavailable (tests, web-only preview).
 */
export function useBudgetPoll(intervalMs = 5000): void {
  useEffect(() => {
    let cancelled = false;

    async function tick() {
      try {
        const status = await getBudgetStatus();
        if (cancelled) return;
        useAiStore.setState((state) => ({
          budget: {
            ...state.budget,
            usedCents: status.used_today_cents,
            limitCents: status.limits.daily_cents ?? state.budget.limitCents,
            periodStartedAt: status.day_started_at,
          },
        }));
      } catch {
        // no Tauri backend — quiet fail
      }
    }

    tick();
    const id = window.setInterval(tick, intervalMs);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [intervalMs]);
}
