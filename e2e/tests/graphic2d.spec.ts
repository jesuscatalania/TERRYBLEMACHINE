import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

test.describe("Graphic 2D", () => {
  test("renders the page", async ({ page }) => {
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

    await page.goto("/graphic2d");
    await expect(page.getByText(/MOD—02 · 2D GRAPHIC/i)).toBeVisible();
    await expect(page.getByLabel(/Describe the image/i)).toBeVisible();
  });
});
