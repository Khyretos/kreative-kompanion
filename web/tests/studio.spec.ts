import { test, expect } from "@playwright/test";

// STU-01: the Studio section: pick a type, describe it, pick a size, make 1 or 4; your images.
test("Studio makes images from a type, a description and a size", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="studio"]');
  const studio = page.locator("#studio");
  await expect(studio.locator("h1")).toHaveText("Studio");
  const types = studio.locator('.studio-types [role="radio"]');
  await expect(types.locator("strong")).toHaveText(["Character", "Scene", "Landscape", "Sprite", "Icon", "OC sheet", "Video", "Music", "Sound effect"]);
  await expect(types.first()).toHaveAttribute("aria-checked", "true");
  const sizes = studio.locator('.studio-form [aria-label="Size"] button');
  await expect(sizes).toHaveText(["Tall", "Square"]);
  await expect(sizes.first()).toHaveAttribute("aria-pressed", "true");
  // Landscape: wide first.
  await types.nth(2).click();
  await expect(types.nth(2)).toHaveAttribute("aria-checked", "true");
  await expect(sizes).toHaveText(["Wide", "Square", "Tall"]);
  await expect(sizes.first()).toHaveAttribute("aria-pressed", "true");
  // The demo library holds one finished image and one failed run (STU-D1: Clear failed).
  const runs = studio.locator(".studio-runs > li");
  await expect(runs).toHaveCount(2);
  await expect(runs.first()).toContainText("Character");
  await expect(runs.first().locator("img")).toHaveAttribute("alt", /Character: /);
  // Make 4 landscapes.
  await studio.locator("#studio-prompt").fill("misty mountains at sunrise");
  await studio.locator('.studio-form input[name="four"]').check();
  await studio.locator('.studio-form button[type="submit"]').click();
  await expect(runs).toHaveCount(6);
  await expect(runs.first()).toContainText("Landscape · Wide");
  await expect(runs.first()).toContainText("misty mountains at sunrise");
  await expect(studio.locator(".studio-run.s-done")).toHaveCount(5, { timeout: 8000 });
  // An empty description is refused before anything is sent.
  await studio.locator("#studio-prompt").fill("   ");
  await studio.locator('.studio-form button[type="submit"]').click();
  await expect(studio.locator('.studio-form [role="alert"]')).toHaveText("Describe what to make.");
  // A finished image opens large.
  await runs.first().locator("button.studio-thumb").first().click();
  await expect(page.locator("dialog.studio-lightbox img")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator("dialog.studio-lightbox")).toHaveCount(0);
});
