import { invoke } from "@tauri-apps/api/core";

export type BudgetState = "ok" | "warn" | "block";

export interface BudgetLimits {
  daily_cents?: number | null;
  session_cents?: number | null;
}

export interface BudgetStatus {
  state: BudgetState;
  used_today_cents: number;
  used_session_cents: number;
  limits: BudgetLimits;
  day_started_at: string;
  session_started_at: string;
}

export function getBudgetStatus(): Promise<BudgetStatus> {
  return invoke<BudgetStatus>("get_budget_status");
}

export function setBudgetLimit(limits: BudgetLimits): Promise<BudgetStatus> {
  return invoke<BudgetStatus>("set_budget_limit", { limits });
}

export function exportUsage(): Promise<string> {
  return invoke<string>("export_usage");
}
