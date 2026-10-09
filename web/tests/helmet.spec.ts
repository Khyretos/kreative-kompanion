// STU-C3: "Has a helmet" on the OC sheet makes a pair (helmet on, face visible) shown as one card with a toggle.
import { test, expect, type Page } from "@playwright/test";

test.use({ viewport: { width: 390, height: 844 } });

async function open(page: Page) {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('#conv-head [data-action="pane"][data-pane="left"]');
  await page.click('[data-action="studio"]');
  return page.locator("#studio");
}

test("A helmet description makes one card with a Helmet on / Face toggle", async ({ page }) => {
  const studio = await open(page);
  await expect(studio.locator(".studio-helmet")).toHaveCount(0);
  await studio.locator('.studio-types [role="radio"]', { hasText: "OC sheet" }).click();
  await studio.locator(".studio-helmet summary").click();
  await studio.locator("#studio-helmet").fill("hooded helmet with glowing yellow eyes");
  await studio.locator("#studio-prompt").fill("a rogue in black and red");
  const before = await studio.locator(".studio-runs > li").count();
  await studio.locator('.studio-form button[type="submit"]').click();
  const pair = studio.locator(".studio-pair").first();
  await expect(pair).toBeVisible();
  await expect(studio.locator(".studio-runs > li")).toHaveCount(before + 1);
  await expect(pair.locator('.studio-run[data-helmet="on"]')).toBeVisible();
  await expect(pair.locator('.studio-run[data-helmet="off"]')).toBeHidden();
  await expect(pair.locator('.studio-run[data-helmet="on"]')).toContainText("hooded helmet with glowing yellow eyes");
  await pair.locator('[data-action="studio-side"][data-side="off"]').click();
  await expect(pair.locator('.studio-run[data-helmet="off"]')).toBeVisible();
  await expect(pair.locator('.studio-run[data-helmet="on"]')).toBeHidden();
  await expect(pair.locator('[data-side="off"][data-action="studio-side"]')).toHaveAttribute("aria-pressed", "true");
  // The chosen side stays after the runs finish and the list re-renders.
  await expect(pair.locator('.studio-run[data-helmet="off"] img')).toBeVisible();
  await expect(studio.locator(".studio-pair").first().locator('.studio-run[data-helmet="on"]')).toBeHidden();
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await studio.locator(".studio-helmet").scrollIntoViewIfNeeded();
    await page.screenshot({ path: `test-results/stu-c3-${scheme}.png`, fullPage: false });
  }
});
