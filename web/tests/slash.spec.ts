import { test, expect } from "@playwright/test";

// SLASH (Kees 10-27): "/" commands act at once without sending a message.
test.describe("Slash commands", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/?demo");
    await page.click("button.found-server");
    await page.click('[data-action="open-chat"][data-id="c-kk"]');
  });

  test("typing / lists the commands and filters them", async ({ page }) => {
    await page.fill("#prompt", "/");
    const menu = page.locator("#slash-menu");
    await expect(menu).toBeVisible();
    expect(await menu.locator(".slash-item").count()).toBeGreaterThan(10);
    await page.fill("#prompt", "/eff hi");
    await expect(menu.locator(".slash-item")).toHaveCount(1);
    await expect(menu.locator(".slash-item code")).toHaveText("/effort high");
    await page.press("#prompt", "Escape");
    await expect(menu).toBeHidden();
  });

  test("a command runs without sending: pick a computer with the keyboard", async ({ page }) => {
    const before = await page.locator("article.msg").count();
    await page.fill("#prompt", "/computer sou");
    await page.press("#prompt", "Enter");
    await expect(page.locator("#pc-machine option:checked")).toHaveText("soucouyant");
    await expect(page.locator("#prompt")).toHaveValue("");
    expect(await page.locator("article.msg").count()).toBe(before);
  });

  test("a prompt starter fills the box instead", async ({ page }) => {
    await page.fill("#prompt", "/pla");
    await page.press("#prompt", "Tab");
    await expect(page.locator("#prompt")).toHaveValue(/^Make a step-by-step plan first/);
  });

  test("a path is not a command: Enter sends it", async ({ page }) => {
    await page.fill("#prompt", "/home/kees/notes");
    await expect(page.locator("#slash-menu")).toBeHidden();
    await page.press("#prompt", "Enter");
    await expect(page.locator("article.msg.user").last()).toContainText("/home/kees/notes");
  });
});
