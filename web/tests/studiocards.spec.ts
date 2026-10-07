import { test, expect, type Page } from "@playwright/test";

// STU-UI1 (Kees, 2026-10-07): uniform type cards (title + badges), the chosen type's details beside them.
async function openStudio(page: Page): Promise<void> {
  await page.goto("/?demo");
  await page.click("button.found-server");
  const menu = page.locator('#conv-head [data-action="pane"][data-pane="left"]');
  if ((page.viewportSize()?.width ?? 1280) < 760) await menu.click();
  await page.click('[data-action="studio"]');
  await expect(page.locator('#studio .studio-types [role="radio"]').first()).toBeVisible();
}

test("Studio type cards are the same height and show badges, not text", async ({ page }) => {
  await openStudio(page);
  const cards = page.locator('#studio .studio-types [role="radio"]');
  const heights = await cards.evaluateAll((els) => els.map((e) => Math.round(e.getBoundingClientRect().height)));
  expect(heights.length).toBeGreaterThan(5);
  expect(new Set(heights).size).toBe(1);
  await expect(page.locator("#studio .studio-types .studio-warning")).toHaveCount(0);
  const oc = page.locator('#studio .studio-type[data-type="oc-sheet"] .studio-badge');
  await expect(oc.first()).toHaveText("Image");
  await expect(page.locator('#studio .studio-type[data-type="oc-sheet"]')).toContainText("Face");
  await expect(page.locator('#studio .studio-type[data-type="oc-sheet"]')).toContainText("Rating");
  await expect(page.locator('#studio .studio-type[data-type="video"] .studio-badge').first()).toHaveText("Video");
  await expect(page.locator('#studio .studio-type[data-type="music"] .studio-badge').first()).toHaveText("Audio");
});

test("The detail panel shows the selected type's hint, sizes and warning", async ({ page }) => {
  await openStudio(page);
  const detail = page.locator("#studio-type-detail");
  await expect(detail.locator("h3")).toHaveText("Character");
  await expect(detail.locator(".studio-warning")).toHaveCount(0);
  await page.click('#studio .studio-type[data-type="oc-sheet"]');
  await expect(detail.locator("h3")).toHaveText("OC sheet");
  await expect(detail).toContainText("Character turnaround");
  await expect(detail).toContainText("Wide");
  await expect(detail).toContainText("a face photo");
  await expect(detail.locator(".studio-warning")).toContainText("Licence warning: novaAnimeXL_ilV170.safetensors");
  // Beside the cards on a wide screen.
  const grid = await page.locator("#studio .studio-types").boundingBox();
  const box = await detail.boundingBox();
  expect(box!.x).toBeGreaterThanOrEqual(grid!.x + grid!.width - 1);
});

test("Arrow keys, Home and End move the selection; only the selected card is in the tab order", async ({ page }) => {
  await openStudio(page);
  const cards = page.locator('#studio .studio-types [role="radio"]');
  const n = await cards.count();
  await expect(cards.nth(0)).toHaveAttribute("tabindex", "0");
  await expect(cards.nth(1)).toHaveAttribute("tabindex", "-1");
  await cards.nth(0).focus();
  await page.keyboard.press("ArrowRight");
  await expect(cards.nth(1)).toHaveAttribute("aria-checked", "true");
  await expect(cards.nth(1)).toBeFocused();
  await expect(page.locator("#studio-type-detail h3")).toHaveText("Scene");
  await page.keyboard.press("ArrowDown");
  await expect(cards.nth(2)).toHaveAttribute("aria-checked", "true");
  await page.keyboard.press("ArrowLeft");
  await expect(cards.nth(1)).toBeFocused();
  await page.keyboard.press("ArrowUp");
  await expect(cards.nth(0)).toHaveAttribute("aria-checked", "true");
  await expect(cards.nth(0)).toBeFocused();
  await page.keyboard.press("ArrowLeft");
  await expect(cards.nth(n - 1)).toHaveAttribute("aria-checked", "true");
  await expect(cards.nth(n - 1)).toBeFocused();
  await page.keyboard.press("Home");
  await expect(cards.nth(0)).toHaveAttribute("aria-checked", "true");
  await expect(cards.nth(0)).toBeFocused();
  await page.keyboard.press("End");
  await expect(cards.nth(n - 1)).toHaveAttribute("aria-checked", "true");
  await expect(cards.nth(n - 1)).toBeFocused();
});

test("On a phone the detail panel sits under the cards", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openStudio(page);
  const grid = await page.locator("#studio .studio-types").boundingBox();
  const box = await page.locator("#studio-type-detail").boundingBox();
  expect(box!.y).toBeGreaterThanOrEqual(grid!.y + grid!.height - 1);
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
});
