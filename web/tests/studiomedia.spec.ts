import { test, expect } from "@playwright/test";

// STU-02: video, music and sound effects in the Studio section, with players in "Your results".
test("Studio makes music and sound effects and plays results", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="studio"]');
  const studio = page.locator("#studio");
  const types = studio.locator('.studio-types [role="radio"]');
  await expect(types.locator("strong")).toHaveText(["Character", "Scene", "Landscape", "Sprite", "Icon", "OC sheet", "Video", "Music", "Sound effect"]);
  // Music: no sizes and no Make 4; a length and optional lyrics.
  await types.filter({ hasText: "Music" }).click();
  await expect(studio.locator('.studio-form [aria-label="Size"]')).toHaveCount(0);
  await expect(studio.locator('.studio-form input[name="four"]')).toHaveCount(0);
  await expect(studio.locator("#studio-seconds")).toHaveValue("30");
  await expect(studio.locator("#studio-seconds")).toHaveAttribute("max", "240");
  await expect(studio.locator("#studio-lyrics")).toBeVisible();
  await studio.locator("#studio-prompt").fill("calm lofi piano loop");
  await studio.locator("#studio-seconds").fill("12");
  await studio.locator('.studio-form button[type="submit"]').click();
  const runs = studio.locator(".studio-runs > li");
  await expect(runs.first()).toContainText("Music · 12 s");
  await expect(runs.first().locator("audio")).toHaveCount(1, { timeout: 8000 });
  await expect(runs.first().locator('a[download]')).toHaveText("WAV");
  // Sound effect: up to 30 s, no lyrics.
  await types.filter({ hasText: "Sound effect" }).click();
  await expect(studio.locator("#studio-seconds")).toHaveValue("4");
  await expect(studio.locator("#studio-seconds")).toHaveAttribute("max", "30");
  await expect(studio.locator("#studio-lyrics")).toHaveCount(0);
  // Video: sizes, no Make 4; a finished video plays inline.
  await types.filter({ hasText: "Video" }).click();
  await expect(studio.locator('.studio-form [aria-label="Size"] button')).toHaveText(["Wide", "Tall", "Square"]);
  await expect(studio.locator('.studio-form input[name="four"]')).toHaveCount(0);
  await studio.locator("#studio-prompt").fill("a fox running through snow");
  await studio.locator('.studio-form button[type="submit"]').click();
  await expect(runs.first()).toContainText("Video · Wide");
  await expect(runs.first().locator("video")).toHaveCount(1, { timeout: 8000 });
});
