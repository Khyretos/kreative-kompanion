import { test, expect, type Page } from "@playwright/test";

async function openDemo(page: Page): Promise<void> {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await expect(page.locator("#left")).toBeVisible();
}

test.describe("Settings page", () => {
  test.beforeEach(async ({ page }) => {
    await openDemo(page);
  });

  test("opens embedded", async ({ page }) => {
    await page.click('[data-action="settings"]');
    await expect(page.locator("#settings")).toBeVisible();
    await expect(page.locator("#messages")).toBeHidden();
  });

  test("Escape returns to chat", async ({ page }) => {
    await page.click('[data-action="settings"]');
    await expect(page.locator("#settings")).toBeVisible();
    
    await page.keyboard.press("Escape");
    await expect(page.locator("#settings")).toBeHidden();
    await expect(page.locator("#messages")).toBeVisible();
  });

  test("browser back returns to chat", async ({ page }) => {
    await page.click('[data-action="settings"]');
    await expect(page.locator("#settings")).toBeVisible();
    
    await page.goBack();
    await expect(page.locator("#settings")).toBeHidden();
    await expect(page.locator("#messages")).toBeVisible();
  });

  test("unsaved edits ask first", async ({ page }) => {
    page.removeAllListeners("dialog");
    await page.click('[data-action="settings"]');
    await expect(page.locator("#settings")).toBeVisible();
    await page.locator("#notify-email").fill("x@example.com");

    const asked: string[] = [];
    page.once("dialog", (d) => { 
      asked.push(d.message()); 
      void d.dismiss(); 
    });
    await page.keyboard.press("Escape");
    await expect.poll(() => asked.length).toBe(1);
    await expect(page.locator("#settings")).toBeVisible();

    page.once("dialog", (d) => { 
      asked.push(d.message()); 
      void d.accept(); 
    });
    await page.keyboard.press("Escape");
    await expect(page.locator("#settings")).toBeHidden();
    expect(asked).toEqual(["Leave Settings without saving your changes?", "Leave Settings without saving your changes?"]);
  });
});

test.describe("Settings page on a phone", () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test.beforeEach(async ({ page }) => {
    await openDemo(page);
  });

  test("phone menu button opens the sidebar", async ({ page }) => {
    await page.click('#conv-head [data-action="pane"][data-pane="left"]');
    await page.click('[data-action="settings"]');
    await expect(page.locator("#settings [data-action=\"pane\"][data-pane=\"left\"]")).toBeVisible();
    
    await page.click('#settings [data-action="pane"][data-pane="left"]');
    await expect(page.locator(".shell")).toHaveAttribute("data-pane", "left");
  });
});
