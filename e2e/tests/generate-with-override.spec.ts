import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

/**
 * Asserts the `/flux` slug override flow in Graphic2D:
 *   1) the user types `/flux a sunset` and hits Generate
 *   2) the parsed slug `flux` resolves to Model::FalFluxPro
 *   3) `cleanPrompt` "a sunset" is dispatched (slug stripped)
 *   4) `optimize_prompt` is NOT invoked (Optimize toggle is OFF by default,
 *      which is the "user-only override" smoke we care about here)
 *
 * `installInvokeMock` only crosses serializable JSON, so we cannot pass a
 * function-valued mock to capture the `generate_variants` payload. Instead
 * we layer a second `addInitScript` that wraps the mock's `invoke` and
 * pushes every `(cmd, args)` pair into a `window.__invokeLog` array, which
 * the test reads back via `page.evaluate` after the click.
 */
test.describe("Graphic2D — /tool override", () => {
  test("`/flux a sunset` dispatches FalFluxPro with cleanPrompt", async ({ page }) => {
    await installInvokeMock(page, {
      list_projects: [],
      get_budget_status: {
        state: "ok",
        used_today_cents: 0,
        used_session_cents: 0,
        limits: { daily_cents: null, session_cents: null },
        day_started_at: "2026-01-01T00:00:00Z",
        session_started_at: "2026-01-01T00:00:00Z",
      },
      get_queue_status: { pending: 0, in_flight: 0 },
      generate_variants: [
        {
          url: "data:image/png;base64,iVBOR",
          width: 512,
          height: 512,
          seed: 1,
          model: "FalFluxPro",
          cached: false,
        },
      ],
    });

    // Wrap the mock's invoke (installed by installInvokeMock) so we can
    // observe argument shape from the test side. Registered AFTER
    // installInvokeMock so it sees the wrapped implementation.
    await page.addInitScript(() => {
      const internals = window as unknown as {
        __TAURI_INTERNALS__?: { invoke?: (cmd: string, args?: unknown) => Promise<unknown> };
        __invokeLog?: Array<{ cmd: string; args: unknown }>;
      };
      internals.__invokeLog = [];
      const original = internals.__TAURI_INTERNALS__?.invoke;
      if (!original || !internals.__TAURI_INTERNALS__) return;
      internals.__TAURI_INTERNALS__.invoke = async (cmd: string, args?: unknown) => {
        internals.__invokeLog?.push({ cmd, args });
        return original(cmd, args);
      };
    });

    await page.goto("/graphic2d");
    await expect(page.getByText(/MOD—02 · 2D GRAPHIC/i)).toBeVisible();

    const promptInput = page.getByLabel(/Describe the image/i);
    await promptInput.fill("/flux a sunset");
    await page.getByRole("button", { name: /Generate 4 variants/i }).click();

    // The "Generated N variants" success toast confirms the round-trip
    // completed without falling into the catch branch.
    await expect(page.getByText(/Generated 1 variants/i)).toBeVisible();

    const log = await page.evaluate(() => {
      const w = window as unknown as { __invokeLog?: Array<{ cmd: string; args: unknown }> };
      return w.__invokeLog ?? [];
    });

    const generateCalls = log.filter((e) => e.cmd === "generate_variants");
    expect(generateCalls).toHaveLength(1);
    const payload = generateCalls[0].args as { input?: Record<string, unknown> };
    expect(payload.input).toMatchObject({
      prompt: "a sunset",
      model_override: "FalFluxPro",
      module: "graphic2d",
    });

    // Optimize toggle defaults OFF, so Claude must NOT have been pinged.
    const optimizeCalls = log.filter((e) => e.cmd === "optimize_prompt");
    expect(optimizeCalls).toHaveLength(0);
  });
});
