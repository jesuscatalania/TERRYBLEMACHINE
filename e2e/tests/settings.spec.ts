import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

test.describe("Settings modal", () => {
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
      list_api_keys: ["claude", "fal"],
    });
  });

  test("header gear icon opens Settings and lists all 9 providers", async ({ page }) => {
    await page.goto("/");
    // Header has its own Settings button; the sidebar has a second one at the
    // bottom. Either should work — pick the first match.
    await page
      .getByRole("button", { name: /^settings$/i })
      .first()
      .click();

    const dialog = page.getByRole("dialog", { name: /settings .* api keys/i });
    await expect(dialog).toBeVisible();

    for (const label of [
      "Anthropic Claude",
      // Kling direct is now optional — fal.ai handles the default Kling
      // routing, so the label is flagged as such.
      "Kling AI Video (direct — optional, fal handles Kling by default)",
      "Runway Gen-3",
      "Higgsfield Video",
      "Shotstack (timeline assembly)",
      "Ideogram (logos / typography)",
      "Meshy 3D",
      "fal.ai (images + Kling video)",
      "Replicate (specialty models)",
    ]) {
      await expect(dialog.getByText(label)).toBeVisible();
    }
  });

  test("Escape closes the Settings modal", async ({ page }) => {
    await page.goto("/");
    await page
      .getByRole("button", { name: /^settings$/i })
      .first()
      .click();
    const dialog = page.getByRole("dialog", { name: /settings .* api keys/i });
    await expect(dialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(dialog).not.toBeVisible();
  });
});
