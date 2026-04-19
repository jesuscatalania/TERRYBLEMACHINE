import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

test.describe("Welcome onboarding", () => {
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
    // installInvokeMock's addInitScript pre-sets the welcome-dismissed flag
    // so other specs don't have to deal with the modal. We register a SECOND
    // addInitScript AFTER it so the removal runs last on every navigation,
    // exposing the modal for this suite. Playwright executes initScripts in
    // registration order (see invoke-mock.ts comment above).
    await page.addInitScript(() => {
      window.localStorage.removeItem("tm:welcome:dismissed");
    });
  });

  test("modal appears on first launch and dismisses on Done", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("dialog", { name: /Welcome to TERRYBLEMACHINE/i })).toBeVisible();
    await page.getByRole("button", { name: /^Next$/i }).click();
    await page.getByRole("button", { name: /^Next$/i }).click();
    await page.getByRole("button", { name: /^Done$/i }).click();
    await expect(page.getByRole("dialog", { name: /Welcome/i })).not.toBeVisible();
  });

  test("Skip dismisses without going through steps", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("dialog", { name: /Welcome to TERRYBLEMACHINE/i })).toBeVisible();
    await page.getByRole("button", { name: /Skip/i }).click();
    await expect(page.getByRole("dialog", { name: /Welcome/i })).not.toBeVisible();
  });
});
