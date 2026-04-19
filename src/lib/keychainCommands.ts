import { invoke } from "@tauri-apps/api/core";

/**
 * Mirrors `KeyStoreIpcError` in `src-tauri/src/keychain/commands.rs`.
 * Serde writes the enum as `{ kind, detail }` where `kind` is the
 * PascalCase variant name and `detail` is the string payload.
 */
export interface KeyStoreIpcError {
  kind: "NotFound" | "InvalidService" | "Keychain";
  detail: string;
}

export function isKeyStoreIpcError(err: unknown): err is KeyStoreIpcError {
  return (
    typeof err === "object" &&
    err !== null &&
    "kind" in err &&
    "detail" in err &&
    typeof (err as { kind: unknown }).kind === "string" &&
    typeof (err as { detail: unknown }).detail === "string"
  );
}

/** Persist an API key for `service` in the OS keychain. */
export function storeApiKey(service: string, key: string): Promise<void> {
  return invoke<void>("store_api_key", { service, key });
}

/**
 * Read the stored key for `service`. Rejects with a `KeyStoreIpcError`
 * whose `kind` is `"NotFound"` when no key is configured.
 */
export function getApiKey(service: string): Promise<string> {
  return invoke<string>("get_api_key", { service });
}

/** Remove the key for `service` (idempotent at the backend level). */
export function deleteApiKey(service: string): Promise<void> {
  return invoke<void>("delete_api_key", { service });
}

/** List the service IDs that currently have a key stored. */
export function listApiKeys(): Promise<string[]> {
  return invoke<string[]>("list_api_keys");
}
