// STU-01c: Studio ratings: only an account with the adult-content right sees Questionable and Explicit.
import { test, expect, type Page } from "@playwright/test";

async function open(page: Page) {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="studio"]');
  return page.locator("#studio");
}

test("A normal account sees only General and Sensitive", async ({ page }) => {
  const studio = await open(page);
  const rating = studio.locator("#studio-rating");
  await expect(rating).toHaveCount(0);
  await studio.locator('.studio-types [role="radio"]', { hasText: "OC sheet" }).click();
  await expect(rating.locator("option")).toHaveText(["General", "Sensitive"]);
});

test("An adult account picks Explicit and keeps it", async ({ page }) => {
  await page.addInitScript(() => { (globalThis as { __kkDemoAdult?: boolean }).__kkDemoAdult = true; });
  const studio = await open(page);
  const rating = studio.locator("#studio-rating");
  await studio.locator('.studio-types [role="radio"]', { hasText: "OC sheet" }).click();
  await expect(rating.locator("option")).toHaveText(["General", "Sensitive", "Questionable", "Explicit"]);
  await rating.selectOption("explicit");
  await studio.locator('.studio-form input[name="four"]').check();
  await expect(rating).toHaveValue("explicit");
});
