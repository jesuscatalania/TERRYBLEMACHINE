import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

test.describe("Navigation", () => {
  test.beforeEach(async ({ page }) => {
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
  });

  test("/ redirects to /website and shows the Website builder", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText(/MOD—01 · WEBSITE BUILDER/i)).toBeVisible();
  });

  test("module sidebar switches the active page", async ({ page }) => {
    await page.goto("/");
    // Sidebar items are <button> (not <a>); label is "Type & Logo" (per
    // src/components/shell/modules.ts), not "typography".
    await page.getByRole("button", { name: /type & logo/i }).click();
    await expect(page).toHaveURL(/\/typography/);
    await expect(page.getByText(/MOD—05 · TYPE & LOGO/i)).toBeVisible();
  });

  test("deep-link /typography lands directly", async ({ page }) => {
    await page.goto("/typography");
    await expect(page.getByText(/MOD—05 · TYPE & LOGO/i)).toBeVisible();
  });

  test("unknown route redirects to /website", async ({ page }) => {
    await page.goto("/nonexistent");
    await expect(page.getByText(/MOD—01 · WEBSITE BUILDER/i)).toBeVisible();
  });
});
