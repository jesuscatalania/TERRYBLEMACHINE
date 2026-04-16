import { useEffect } from "react";
import { useLocation } from "react-router-dom";
import { pathToModule } from "@/lib/moduleRoutes";
import { useAppStore } from "@/stores/appStore";

/**
 * Keeps `useAppStore.activeModule` in sync with the current URL. Mount inside
 * the router tree. Runs once on mount and on every `location.pathname` change
 * (covers back/forward navigation + programmatic `navigate()` calls).
 */
export function useModuleRouteSync(): void {
  const { pathname } = useLocation();
  const setActiveModule = useAppStore((s) => s.setActiveModule);

  useEffect(() => {
    const mod = pathToModule(pathname);
    if (mod) setActiveModule(mod);
  }, [pathname, setActiveModule]);
}
