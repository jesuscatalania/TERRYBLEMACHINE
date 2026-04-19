import { Eye, EyeOff, Trash2 } from "lucide-react";
import { useEffect, useId, useState } from "react";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { LoadingButton } from "@/components/ui/LoadingButton";
import {
  type ClaudeTransport,
  detectClaudeCli,
  getClaudeTransport,
  setClaudeTransport,
} from "@/lib/claudeTransport";
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

  const hasTransports = Boolean(provider.transports && provider.transports.length > 0);
  const [transport, setTransportState] = useState<ClaudeTransport>("auto");
  const [cliPath, setCliPath] = useState<string | null>(null);

  useEffect(() => {
    if (!hasTransports) return;
    let cancelled = false;
    (async () => {
      try {
        const [current, path] = await Promise.all([getClaudeTransport(), detectClaudeCli()]);
        if (cancelled) return;
        setTransportState(current);
        setCliPath(path);
      } catch (err) {
        if (cancelled) return;
        notify({
          kind: "error",
          message: `Reading ${provider.label} transport failed`,
          detail: String(err),
        });
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [hasTransports, notify, provider.label]);

  async function handleTransportChange(next: ClaudeTransport) {
    const prev = transport;
    setTransportState(next);
    try {
      await setClaudeTransport(next);
      notify({
        kind: "success",
        message: `${provider.label} transport set to ${next}`,
      });
    } catch (err) {
      setTransportState(prev);
      notify({
        kind: "error",
        message: `Setting ${provider.label} transport failed`,
        detail: isKeyStoreIpcError(err) ? err.detail : String(err),
      });
    }
  }

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
        {hasTransports ? (
          <select
            data-testid={`provider-transport-${provider.id}`}
            aria-label={`${provider.label} transport`}
            value={transport}
            onChange={(e) => handleTransportChange(e.target.value as ClaudeTransport)}
            className="rounded border border-neutral-dark-600 bg-neutral-dark-800 px-1 py-0.5 font-mono text-2xs text-neutral-dark-100 uppercase tracking-label"
          >
            {provider.transports?.map((t) => (
              <option key={t} value={t}>
                {t}
              </option>
            ))}
          </select>
        ) : null}
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
      {hasTransports ? (
        <p
          data-testid={`provider-cli-status-${provider.id}`}
          className="font-mono text-2xs text-neutral-dark-500"
        >
          {cliPath
            ? `CLI detected at ${cliPath}`
            : "No claude CLI binary detected — run `brew install anthropic/claude-code/claude`"}
        </p>
      ) : null}
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
