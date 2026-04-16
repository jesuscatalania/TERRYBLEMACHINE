import { useEffect } from "react";
import { listProjects } from "@/lib/projectCommands";
import { useProjectStore } from "@/stores/projectStore";

/**
 * Runs once on mount: fetches the on-disk project list and seeds
 * `useProjectStore.recents` (capped at 10 by the store).
 *
 * Fails silently when the Tauri backend is unavailable (e.g. in Vitest).
 */
export function useProjectsBoot(): void {
  const hydrateRecents = useProjectStore((s) => s.hydrateRecents);

  useEffect(() => {
    let cancelled = false;
    listProjects()
      .then((projects) => {
        if (!cancelled) hydrateRecents(projects);
      })
      .catch(() => {
        // No Tauri backend (tests, web preview) — ignore.
      });
    return () => {
      cancelled = true;
    };
  }, [hydrateRecents]);
}
