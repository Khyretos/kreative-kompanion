import { test, expect } from "@playwright/test";

// CHAT-01: a chat turns MCP tool servers on (Tools menu next to the effort chip); new chats keep the pick.
test("the Tools menu turns tool servers on for a new chat and keeps them after the first message", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('#left .new-chat');
  const chip = page.locator(".tools-chip");
  await expect(chip).toHaveText("Tools off ▾");
  await chip.click();
  const items = page.locator('.tools-menu [role="menuitemcheckbox"]');
  await expect(items.locator(".effort-name")).toHaveText(["Stack Overflow", "Developer docs"]);
  await items.first().click();
  await expect(items.first().locator(".effort-hint")).toHaveText("3.5 million answered programming questions");
  await expect(items.first()).toHaveAttribute("aria-checked", "true");
  await expect(chip).toHaveText("Tools: 1 ▾");
  await page.keyboard.press("Escape");
  await expect(page.locator(".tools-menu")).toHaveCount(0);
  // UI-02: a click outside closes it, like the Effort chip; Web is its own chip, not in the list.
  await chip.click();
  await page.click("#prompt");
  await expect(page.locator(".tools-menu")).toHaveCount(0);
  const web = page.locator(".web-chip");
  await expect(web).toHaveText("Web off");
  await web.click();
  await expect(web).toHaveAttribute("aria-pressed", "true");
  await expect(chip).toHaveText("Tools: 1 ▾");
  await page.fill("#prompt", "How do I read a file in Rust?");
  await page.press("#prompt", "Enter");
  await expect(page.locator("#conv-head h1")).toHaveText("How do I read a file in Rust?");
  await expect(chip).toHaveText("Tools: 1 ▾");
});

test("an existing chat keeps its own tools", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  const first = page.locator('#left [data-action="open-chat"]').first();
  await first.click();
  const chip = page.locator(".tools-chip");
  await expect(chip).toHaveText("Tools off ▾");
  await chip.click();
  await page.locator('.tools-menu [role="menuitemcheckbox"]', { hasText: "Developer docs" }).click();
  await expect(chip).toHaveText("Tools: 1 ▾");
  await page.locator('#left [data-action="open-chat"]').nth(1).click();
  await expect(chip).toHaveText("Tools off ▾");
  await first.click();
  await expect(chip).toHaveText("Tools: 1 ▾");
});
