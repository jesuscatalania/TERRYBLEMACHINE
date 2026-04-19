/**
 * Converts an unknown rejection value into a human-readable string.
 *
 * Tauri IPC commands return typed Rust errors like `VideoIpcError` which
 * serialize as `{ kind: "router", detail: "connection timeout" }` via
 * `#[serde(tag = "kind", content = "detail")]`. These arrive on the
 * frontend as plain objects — `instanceof Error` is false, and
 * `String({...})` yields `"[object Object]"`. This helper recognizes
 * the tagged shape and returns readable text.
 */
export function formatError(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  if (typeof err === "object" && err !== null) {
    const obj = err as { kind?: unknown; detail?: unknown; message?: unknown };
    if (typeof obj.message === "string") return obj.message;
    const kindStr = typeof obj.kind === "string" ? obj.kind : null;
    const detailStr = typeof obj.detail === "string" ? obj.detail : null;
    if (kindStr && detailStr) return `${kindStr}: ${detailStr}`;
    if (detailStr) return detailStr;
    if (kindStr) return kindStr;
    try {
      return JSON.stringify(err);
    } catch {
      return "unknown error";
    }
  }
  return String(err);
}
