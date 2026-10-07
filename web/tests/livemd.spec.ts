import { test, expect } from "@playwright/test";

test.describe("Live markdown in the composer (CHAT-07)", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/?demo");
    await page.click("button.found-server");
  });

  test("styles lists, quotes, code, tables and bold while typing; the raw text stays the value", async ({ page }) => {
    const raw = "# Plan\n- one **bold**\n> quoted\n`inline`\n| a | b |\n|---|---|\n```\ncode\n```";
    await page.fill("#prompt", raw);
    const m = page.locator(".lm-mirror");
    await expect(m.locator(".lm-h")).toHaveCount(1);
    await expect(m.locator(".lm-li")).toHaveCount(1);
    await expect(m.locator(".lm-b")).toHaveText("**bold**");
    await expect(m.locator(".lm-quote")).toHaveCount(1);
    await expect(m.locator(".lm-code")).toHaveText("`inline`");
    await expect(m.locator(".lm-row")).toHaveCount(2);
    await expect(m.locator(".lm-fence")).toHaveCount(3);
    await expect(m).toHaveText(raw);
    await expect(page.locator("#prompt")).toHaveValue(raw);
  });

  test("Shift+Enter adds a line, Enter sends and clears the mirror", async ({ page }) => {
    await page.fill("#prompt", "hello");
    await page.press("#prompt", "Shift+Enter");
    await page.keyboard.type("- item");
    await expect(page.locator("#prompt")).toHaveValue("hello\n- item");
    await expect(page.locator(".lm-mirror .lm-li")).toHaveCount(1);
    await page.press("#prompt", "Enter");
    await expect(page.locator("#prompt")).toHaveValue("");
    await expect(page.locator(".lm-mirror")).toHaveText("");
  });

  test("the mirror escapes html", async ({ page }) => {
    await page.fill("#prompt", "<img src=x onerror=alert(1)> **b**");
    await expect(page.locator(".lm-mirror img")).toHaveCount(0);
    await expect(page.locator(".lm-mirror")).toContainText("<img src=x");
  });
});
