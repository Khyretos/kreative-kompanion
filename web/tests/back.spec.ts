import { expect, test } from "@playwright/test";

// BUG-05: Android back goes to the chat first; the last chat's menu stays above the sidebar footer.
test.use({ viewport: { width: 390, height: 844 } });

const back = (page: import("@playwright/test").Page) =>
  page.evaluate(() => (window as unknown as { kompanionBack: () => boolean }).kompanionBack());

test("back from Studio goes to the chat, then lets the app close", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('#conv-head [data-action="pane"][data-pane="left"]');
  await page.click('[data-action="studio"]');
  await expect(page.locator("#studio h1")).toBeVisible();
  expect(await back(page)).toBe(true);
  await expect(page.locator("#studio")).toBeHidden();
  await expect(page.locator(".shell")).toHaveAttribute("data-pane", "main");
  expect(await back(page)).toBe(false);
});

test("back closes the sidebar first", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('#conv-head [data-action="pane"][data-pane="left"]');
  await expect(page.locator(".shell")).toHaveAttribute("data-pane", "left");
  expect(await back(page)).toBe(true);
  await expect(page.locator(".shell")).toHaveAttribute("data-pane", "main");
});

test("the last chat's menu shows whole above the sidebar footer", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('#conv-head [data-action="pane"][data-pane="left"]');
  const last = page.locator('.nav [data-action="chat-menu"]').last();
  await last.scrollIntoViewIfNeeded();
  await last.click();
  const menu = page.locator(".chat-row.menu-open .menu");
  await expect(menu).toBeVisible();
  const m = (await menu.boundingBox())!;
  const nav = (await page.locator(".left .nav").boundingBox())!;
  expect(m.y).toBeGreaterThanOrEqual(nav.y - 1);
  expect(m.y + m.height).toBeLessThanOrEqual(nav.y + nav.height + 1);
  await page.screenshot({ path: "test-results/bug05-menu.png" });
  expect(await back(page)).toBe(true);
  await expect(menu).toBeHidden();
});
