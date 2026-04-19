import { Eye, EyeOff, Trash2 } from "lucide-react";
import { useId, useState } from "react";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { LoadingButton } from "@/components/ui/LoadingButton";
import { deleteApiKey, isKeyStoreIpcError, storeApiKey } from "@/lib/keychainCommands";
import { useUiStore } from "@/stores/uiStore";
import type { ProviderDef } from "./providers";

export interface ProviderKeyRowProps {
  provider: ProviderDef;
  /** True when the keychain currently holds a key for this service. */
  configured: boolean;
  /** Fired after a successful save / delete so the parent can refresh. */
  onChange: () => void;
}

export function ProviderKeyRow({ provider, configured, onChange }: ProviderKeyRowProps) {
  const inputId = useId();
  const [value, setValue] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [busy, setBusy] = useState(false);
  const notify = useUiStore((s) => s.notify);

  const trimmed = value.trim();

  async function handleSave() {
    if (!trimmed) return;
    setBusy(true);
    try {
      await storeApiKey(provider.id, trimmed);
      setValue("");
      setShowKey(false);
      notify({
        kind: "success",
        message: `Saved ${provider.label} key`,
      });
      onChange();
    } catch (err) {
      notify({
        kind: "error",
        message: `Saving ${provider.label} key failed`,
        detail: isKeyStoreIpcError(err) ? err.detail : String(err),
      });
    } finally {
      setBusy(false);
    }
  }

  async function handleDelete() {
    setBusy(true);
    try {
      await deleteApiKey(provider.id);
      notify({
        kind: "success",
        message: `Removed ${provider.label} key`,
      });
      onChange();
    } catch (err) {
      notify({
        kind: "error",
        message: `Removing ${provider.label} key failed`,
        detail: isKeyStoreIpcError(err) ? err.detail : String(err),
      });
    } finally {
      setBusy(false);
    }
  }

  return (
    <div
      data-testid={`provider-row-${provider.id}`}
      className="flex flex-col gap-2 border-neutral-dark-700 border-b py-3 last:border-b-0"
    >
      <div className="flex items-center gap-2">
        <span
          role="img"
          data-testid={`provider-dot-${provider.id}`}
          data-configured={configured ? "true" : "false"}
          aria-label={configured ? "Configured" : "Not configured"}
          className={`inline-block h-2 w-2 rounded-full ${
            configured ? "bg-emerald-500" : "bg-neutral-dark-600"
          }`}
        />
        <span className="font-mono text-2xs text-neutral-dark-100 uppercase tracking-label">
          {provider.label}
        </span>
        <span className="font-mono text-2xs text-neutral-dark-500">· {provider.plan}</span>
      </div>
      <div className="flex items-center gap-2">
        <div className="flex-1">
          <Input
            id={inputId}
            type={showKey ? "text" : "password"}
            placeholder={
              configured ? "•••••••••• (configured — paste new value to replace)" : "Paste API key"
            }
            value={value}
            onValueChange={setValue}
            disabled={busy}
            aria-label={`${provider.label} API key`}
            autoComplete="off"
            spellCheck={false}
          />
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setShowKey((v) => !v)}
          aria-label={showKey ? `Hide ${provider.label} key` : `Show ${provider.label} key`}
          disabled={busy || !value}
          type="button"
        >
          {showKey ? (
            <EyeOff className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          ) : (
            <Eye className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          )}
        </Button>
        <LoadingButton
          variant="primary"
          size="sm"
          onClick={handleSave}
          disabled={!trimmed}
          loading={busy}
          type="button"
        >
          Save
        </LoadingButton>
        {configured ? (
          <Button
            variant="danger"
            size="sm"
            onClick={handleDelete}
            disabled={busy}
            aria-label={`Remove ${provider.label} key`}
            type="button"
          >
            <Trash2 className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
          </Button>
        ) : null}
      </div>
      <p className="font-mono text-2xs text-neutral-dark-500">
        Get your key at{" "}
        <a
          href={provider.helpUrl}
          target="_blank"
          rel="noreferrer"
          className="text-accent-500 underline hover:text-accent-400"
        >
          {provider.helpUrl}
        </a>
        {provider.hint ? (
          <span className="ml-1 text-neutral-dark-500">· {provider.hint}</span>
        ) : null}
      </p>
    </div>
  );
}
