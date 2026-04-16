import { MODULES } from "@/components/shell/modules";
import type { ModuleId } from "@/stores/appStore";

const MODULE_IDS = MODULES.map((m) => m.id);

/** `"website"` → `"/website"`. Identity mapping today; centralized for future flexibility. */
export function moduleToPath(id: ModuleId): string {
  return `/${id}`;
}

/** Reverse lookup. Returns `undefined` for non-module paths. */
export function pathToModule(pathname: string): ModuleId | undefined {
  const match = pathname.match(/^\/([^/]+)/);
  if (!match) return undefined;
  const first = match[1] as ModuleId;
  return MODULE_IDS.includes(first) ? first : undefined;
}
