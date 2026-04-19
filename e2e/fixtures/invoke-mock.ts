import type { Page } from "@playwright/test";

export type InvokeMock = Record<string, unknown | ((args: unknown) => unknown | Promise<unknown>)>;

/**
 * Patches `window.__TAURI_INTERNALS__.invoke` so the React app's IPC calls
 * resolve against the supplied mock map instead of a real Tauri runtime.
 *
 * Usage in a spec:
 *   await installInvokeMock(page, {
 *     generate_logo_variants: [{ url: "...", local_path: null, seed: 1, model: "ideogram-v3" }],
 *     vectorize_image: { svg: "<svg/>", width: 100, height: 100 },
 *   });
 *   await page.goto("/typography");
 *
 * Unknown command names reject with an explicit error so a spec doesn't
 * silently miss a mock.
 */
export async function installInvokeMock(page: Page, mock: InvokeMock): Promise<void> {
  const serialized = JSON.stringify(serializableMockOnly(mock));
  await page.addInitScript((payload: string) => {
    const map = JSON.parse(payload) as Record<string, unknown>;
    const internals = window as unknown as { __TAURI_INTERNALS__?: Record<string, unknown> };
    internals.__TAURI_INTERNALS__ = internals.__TAURI_INTERNALS__ ?? {};
    (internals.__TAURI_INTERNALS__ as Record<string, unknown>).invoke = async (
      cmd: string,
      _args?: unknown,
    ) => {
      if (!(cmd in map)) {
        throw new Error(`[invoke-mock] unmocked command: ${cmd}`);
      }
      return map[cmd];
    };
    // Pre-dismiss the Welcome modal so it doesn't intercept clicks in every
    // E2E test. The dedicated welcome.spec clears this key explicitly to
    // exercise the onboarding flow.
    try {
      window.localStorage.setItem("tm:welcome:dismissed", "true");
    } catch {
      // localStorage may be unavailable in some pre-navigation contexts; the
      // modal will then surface and the spec can dismiss it manually.
    }
  }, serialized);
}

/**
 * Pure helper: converts the user-facing `InvokeMock` into a JSON-serializable
 * shape, throwing a descriptive error if any value is a function (functions
 * cannot cross the `addInitScript` boundary).
 */
export function serializableMockOnly(mock: InvokeMock): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(mock)) {
    if (typeof v === "function") {
      throw new Error(
        `[invoke-mock] command "${k}" is a function, but addInitScript only crosses serializable values. ` +
          "Use static response shapes; if dynamic responses are needed, file a backlog item to switch to " +
          `Playwright's exposeFunction approach.`,
      );
    }
    out[k] = v;
  }
  return out;
}
