import { useCallback, useState } from "react";

export const WELCOME_LOCALSTORAGE_KEY = "tm:welcome:dismissed";

export interface WelcomeFlowApi {
  open: boolean;
  dismiss: () => void;
}

function isDismissed(): boolean {
  if (typeof window === "undefined") return true; // SSR-safe
  return window.localStorage.getItem(WELCOME_LOCALSTORAGE_KEY) === "true";
}

/**
 * Drives the first-launch onboarding modal. The modal opens automatically
 * unless the user has previously dismissed it (`localStorage` flag).
 */
export function useWelcomeFlow(): WelcomeFlowApi {
  const [open, setOpen] = useState<boolean>(() => !isDismissed());

  const dismiss = useCallback(() => {
    if (typeof window !== "undefined") {
      window.localStorage.setItem(WELCOME_LOCALSTORAGE_KEY, "true");
    }
    setOpen(false);
  }, []);

  return { open, dismiss };
}
