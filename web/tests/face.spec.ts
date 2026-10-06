// STU-01d: the Studio's optional face photo (types with a face graph: OC sheet, VN portrait).
import { test, expect, type Page } from "@playwright/test";

async function open(page: Page) {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="studio"]');
  return page.locator("#studio");
}

test("Only types with a face graph offer a photo", async ({ page }) => {
  const studio = await open(page);
  await expect(studio.locator("#studio-face")).toHaveCount(0);
  await studio.locator('.studio-types [role="radio"]', { hasText: "OC sheet" }).click();
  await expect(studio.locator("#studio-face")).toBeVisible();
  await expect(studio.locator("#studio-face-weight")).toHaveValue("0.85");
});

test("A chosen photo survives a re-render and can be removed", async ({ page }) => {
  const studio = await open(page);
  await studio.locator('.studio-types [role="radio"]', { hasText: "OC sheet" }).click();
  await studio.locator("#studio-face").setInputFiles({ name: "me.png", mimeType: "image/png", buffer: Buffer.from("PNG") });
  await expect(studio.locator(".studio-face")).toContainText("me.png");
  await studio.locator('.studio-form input[name="four"]').check();
  await expect(studio.locator(".studio-face")).toContainText("me.png");
  await studio.locator('[data-action="studio-face-clear"]').click();
  await expect(studio.locator(".studio-face")).not.toContainText("me.png");
});

test("Making with a photo starts a run", async ({ page }) => {
  const studio = await open(page);
  await studio.locator('.studio-types [role="radio"]', { hasText: "OC sheet" }).click();
  await studio.locator("#studio-prompt").fill("a knight with my face");
  await studio.locator("#studio-face").setInputFiles({ name: "me.png", mimeType: "image/png", buffer: Buffer.from("PNG") });
  await studio.locator('.studio-form button[type="submit"]').click();
  await expect(studio.locator(".studio-runs li").first()).toContainText("a knight with my face");
});
