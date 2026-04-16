import { AnimatePresence, motion } from "framer-motion";
import { AlertTriangle, CheckCircle2, Info, X, XCircle } from "lucide-react";
import { useEffect } from "react";
import type { NotificationKind } from "@/stores/uiStore";
import { useUiStore } from "@/stores/uiStore";

const KIND_ICON = {
  success: CheckCircle2,
  error: XCircle,
  warning: AlertTriangle,
  info: Info,
} as const;

const KIND_TONE: Record<NotificationKind, string> = {
  success: "border-emerald-500/40 text-emerald-400",
  error: "border-rose-500/50 text-rose-400",
  warning: "border-amber-400/50 text-amber-300",
  info: "border-neutral-dark-600 text-neutral-dark-200",
};

/**
 * Toaster renders notifications from the uiStore as a fixed stack in the
 * bottom-right corner. Use `useUiStore.getState().notify(...)` to push a toast.
 */
export function Toaster({ autoDismissMs = 5000 }: { autoDismissMs?: number }) {
  const notifications = useUiStore((s) => s.notifications);
  const dismiss = useUiStore((s) => s.dismissNotification);

  // auto-dismiss each toast after `autoDismissMs`
  useEffect(() => {
    if (notifications.length === 0) return;
    const timers: number[] = [];
    for (const n of notifications) {
      timers.push(window.setTimeout(() => dismiss(n.id), autoDismissMs));
    }
    return () => {
      for (const t of timers) window.clearTimeout(t);
    };
  }, [notifications, dismiss, autoDismissMs]);

  if (typeof document === "undefined") return null;

  return (
    <div className="pointer-events-none fixed right-4 bottom-10 z-40 flex flex-col items-end gap-2">
      <AnimatePresence initial={false}>
        {notifications.map((n) => {
          const Icon = KIND_ICON[n.kind];
          return (
            <motion.div
              key={n.id}
              role="status"
              data-kind={n.kind}
              layout
              initial={{ opacity: 0, x: 16 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: 16 }}
              transition={{ duration: 0.16, ease: "easeOut" }}
              className={`pointer-events-auto flex w-80 items-start gap-3 rounded-xs border bg-neutral-dark-900 px-3 py-2.5 shadow-elevated ${KIND_TONE[n.kind]}`}
            >
              <Icon className="mt-0.5 h-4 w-4 shrink-0" strokeWidth={1.5} aria-hidden="true" />
              <div className="flex-1">
                <div className="text-sm text-neutral-dark-50">{n.message}</div>
                {n.detail ? (
                  <div className="mt-0.5 font-mono text-2xs text-neutral-dark-400 tracking-label">
                    {n.detail}
                  </div>
                ) : null}
              </div>
              <button
                type="button"
                aria-label="Dismiss"
                onClick={() => dismiss(n.id)}
                className="grid h-5 w-5 shrink-0 place-items-center rounded-xs text-neutral-dark-400 hover:text-neutral-dark-100"
              >
                <X className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
              </button>
            </motion.div>
          );
        })}
      </AnimatePresence>
    </div>
  );
}
