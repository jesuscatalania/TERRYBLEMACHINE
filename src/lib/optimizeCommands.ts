import { invoke } from "@tauri-apps/api/core";
import type { TaskKind } from "@/lib/toolCatalog";

export const optimizePrompt = (prompt: string, task: TaskKind) =>
  invoke<string>("optimize_prompt", { input: { prompt, task } });
