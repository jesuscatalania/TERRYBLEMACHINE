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

  test("BACKLOG #208 sanity: installInvokeMock pre-sets the welcome flag", async ({ browser }) => {
    // This test does NOT remove the flag — it asserts that installInvokeMock
    // actually sets it. If a regression removes the pre-set, the OTHER specs
    // (typography/navigation/...) will show the welcome modal on /goto. We
    // catch that class of regression here.
    //
    // We open a fresh browser context (not the suite's per-test `page`) so
    // the suite-level beforeEach's flag-removal initScript doesn't apply —
    // we want to observe ONLY installInvokeMock's pre-set behavior.
    const context = await browser.newContext();
    const freshPage = await context.newPage();
    await installInvokeMock(freshPage, {
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
    // Do NOT remove the flag this time
    await freshPage.goto("/");
    // Welcome modal must NOT appear because installInvokeMock pre-dismissed it.
    await expect(freshPage.getByRole("dialog", { name: /Welcome/i })).not.toBeVisible();
    // And the flag is set
    const flag = await freshPage.evaluate(() =>
      window.localStorage.getItem("tm:welcome:dismissed"),
    );
    expect(flag).toBe("true");
    await context.close();
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
