import { expect, test } from "@playwright/test";
import { installInvokeMock } from "../fixtures/invoke-mock";

test.describe("Typography flow", () => {
  test("generate → vectorize → export brand kit", async ({ page }) => {
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
      generate_logo_variants: [
        {
          url: "data:image/png;base64,iVBORw0KGgo=",
          local_path: "/tmp/v1.png",
          seed: 1,
          model: "ideogram-v3",
        },
        {
          url: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUg==",
          local_path: "/tmp/v2.png",
          seed: 2,
          model: "ideogram-v3",
        },
      ],
      vectorize_image: {
        svg: '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect width="100" height="100" fill="#e85d2d"/></svg>',
        width: 100,
        height: 100,
      },
      export_brand_kit: "/tmp/exports/acme-brand-kit.zip",
    });

    await page.goto("/typography");
    await page.getByLabel(/Describe the logo/i).fill("Acme");
    // Two buttons match /Generate/i: the shell Header "Generate" and the
    // Typography header "Generate 6 variants". Click the latter by its
    // specific name so strict-mode doesn't complain.
    await page.getByRole("button", { name: /Generate 6 variants/i }).click();

    const firstVariant = page.locator('[data-testid^="logo-variant-"]').first();
    await expect(firstVariant).toBeVisible();
    // The variant wrapper is a <div>; the clickable element inside has
    // aria-label="Select logo variant". Click that to trigger onSelect.
    await firstVariant.getByRole("button", { name: /Select logo variant/i }).click();

    // Wait for Vectorize to be enabled, then trigger it. Playwright's
    // default click retries against the fabric `upper-canvas` overlay
    // (which intercepts pointer events), so use a programmatic click to
    // avoid pointer-event occlusion without bypassing visibility checks.
    const vectorizeBtn = page.getByRole("button", { name: /^Vectorize$/i });
    await expect(vectorizeBtn).toBeEnabled();
    await vectorizeBtn.evaluate((el) => (el as HTMLButtonElement).click());
    // Wait for the "Vectorized logo" success toast before checking the
    // Export button — vectorize is async (fabric.loadSVGFromString awaits)
    // and the button gates on `vectorized` state flipped after loadSvg.
    await expect(page.getByText(/Vectorized logo/i)).toBeVisible();
    await expect(page.getByRole("button", { name: /Export brand kit/i })).toBeEnabled();

    await page.getByRole("button", { name: /Export brand kit/i }).click();
    await page.getByLabel(/Brand name/i).fill("Acme");
    await page.getByLabel(/Destination directory/i).fill("/tmp/exports");
    await page.getByRole("button", { name: /^Export$/i }).click();

    await expect(page.getByText(/Brand kit exported/i)).toBeVisible();
  });
});
