import { useCallback, useEffect, useState } from "react";
import { Modal } from "@/components/ui/Modal";
import { isKeyStoreIpcError, listApiKeys } from "@/lib/keychainCommands";
import { useUiStore } from "@/stores/uiStore";
import { ProviderKeyRow } from "./ProviderKeyRow";
import { PROVIDERS } from "./providers";

export interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

export function SettingsModal({ open, onClose }: SettingsModalProps) {
  const [configured, setConfigured] = useState<Set<string>>(() => new Set());
  const notify = useUiStore((s) => s.notify);

  const refresh = useCallback(async () => {
    try {
      const list = await listApiKeys();
      setConfigured(new Set(list));
    } catch (err) {
      notify({
        kind: "error",
        message: "Failed to load API key status",
        detail: isKeyStoreIpcError(err) ? err.detail : String(err),
      });
    }
  }, [notify]);

  useEffect(() => {
    if (open) void refresh();
  }, [open, refresh]);

  return (
    <Modal open={open} onClose={onClose} title="Settings — API Keys" maxWidth={640}>
      <div className="flex flex-col gap-2">
        <p className="font-mono text-2xs text-neutral-dark-400">
          Keys are stored in the macOS Keychain. They never leave your machine except to call the
          provider you configure.
        </p>
        <div className="mt-2">
          {PROVIDERS.map((p) => (
            <ProviderKeyRow
              key={p.id}
              provider={p}
              configured={configured.has(p.id)}
              onChange={refresh}
            />
          ))}
        </div>
      </div>
    </Modal>
  );
}
