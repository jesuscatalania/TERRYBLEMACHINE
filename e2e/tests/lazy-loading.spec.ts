import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

test.describe("Lazy module loading", () => {
  test("Suspense fallback renders during cold route entry", async ({ page }) => {
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
    });

    // Slow down per-page chunk requests so the fallback has a visible window.
    // Vite's dev server (used by the Playwright `webServer`) serves source
    // files at `/src/pages/<Page>.tsx`, while a production build serves
    // hashed `/assets/<Page>-<hash>.js` chunks. Match BOTH so this spec
    // works whether tested locally (dev) or in CI against a build preview.
    await page.route(
      /\/(src\/pages|assets)\/(Typography|Graphic2D|Graphic3D|Video|WebsiteBuilder|DesignSystem)([.-][^/]*)?\.(tsx|js)$/,
      async (route) => {
        await new Promise((r) => setTimeout(r, 500));
        await route.continue();
      },
    );

    await page.goto("/");
    // The "/" route redirects to "/website" which lazy-loads WebsiteBuilder.
    // Wait for that initial cold-load to finish so subsequent navigation is
    // clean.
    await expect(page.getByText(/MOD—01 · WEBSITE BUILDER/i)).toBeVisible({
      timeout: 5000,
    });

    // Click navigation to a not-yet-loaded module — fallback should briefly
    // appear because the route handler delays the chunk request 500ms.
    await page.getByRole("button", { name: /type & logo/i }).click();
    await expect(page.getByRole("status", { busy: true })).toBeVisible({
      timeout: 1000,
    });
    // Fallback then resolves to the real page.
    await expect(page.getByText(/MOD—05 · TYPE & LOGO/i)).toBeVisible({
      timeout: 5000,
    });
  });
});
