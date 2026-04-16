import { AnimatePresence, motion } from "framer-motion";
import { X } from "lucide-react";
import { type ReactNode, useEffect } from "react";
import { createPortal } from "react-dom";

export interface ModalProps {
  open: boolean;
  onClose: () => void;
  /** Optional title shown in the modal header. */
  title?: string;
  /** When true (default), Escape + backdrop click close the modal. */
  dismissible?: boolean;
  /** Maximum width in pixels. Defaults to 520. */
  maxWidth?: number;
  children: ReactNode;
  /** Optional footer slot (e.g. action buttons). */
  footer?: ReactNode;
}

export function Modal({
  open,
  onClose,
  title,
  dismissible = true,
  maxWidth = 520,
  children,
  footer,
}: ModalProps) {
  useEffect(() => {
    if (!open || !dismissible) return;
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [open, dismissible, onClose]);

  if (typeof document === "undefined") return null;

  return createPortal(
    <AnimatePresence>
      {open ? (
        <motion.div
          className="fixed inset-0 z-50 flex items-center justify-center"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.12 }}
        >
          {/* Backdrop */}
          <button
            type="button"
            data-testid="modal-backdrop"
            aria-label="Backdrop"
            tabIndex={-1}
            className="absolute inset-0 cursor-default bg-neutral-dark-950/70 backdrop-blur-sm"
            onClick={() => {
              if (dismissible) onClose();
            }}
          />

          {/* Dialog */}
          <motion.div
            role="dialog"
            aria-modal="true"
            aria-labelledby={title ? "modal-title" : undefined}
            initial={{ opacity: 0, scale: 0.98, y: 6 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.98, y: 6 }}
            transition={{ duration: 0.14, ease: "easeOut" }}
            style={{ maxWidth }}
            className="relative z-10 mx-4 w-full rounded-xs border border-neutral-dark-600 bg-neutral-dark-900 shadow-elevated"
          >
            {title ? (
              <div className="flex items-center justify-between border-neutral-dark-700 border-b px-5 py-3">
                <h2
                  id="modal-title"
                  className="font-mono text-2xs text-neutral-dark-300 uppercase tracking-label"
                >
                  {title}
                </h2>
                <button
                  type="button"
                  aria-label="Close"
                  onClick={onClose}
                  className="grid h-5 w-5 place-items-center rounded-xs border border-transparent text-neutral-dark-400 hover:border-neutral-dark-600 hover:text-neutral-dark-100"
                >
                  <X className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
                </button>
              </div>
            ) : (
              <button
                type="button"
                aria-label="Close"
                onClick={onClose}
                className="absolute top-3 right-3 grid h-5 w-5 place-items-center rounded-xs border border-transparent text-neutral-dark-400 hover:border-neutral-dark-600 hover:text-neutral-dark-100"
              >
                <X className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
              </button>
            )}

            <div className="px-5 py-4 text-sm text-neutral-dark-100">{children}</div>

            {footer ? (
              <div className="flex items-center justify-end gap-2 border-neutral-dark-700 border-t px-5 py-3">
                {footer}
              </div>
            ) : null}
          </motion.div>
        </motion.div>
      ) : null}
    </AnimatePresence>,
    document.body,
  );
}
