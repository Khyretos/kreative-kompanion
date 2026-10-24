import { test, expect } from "@playwright/test";

// CHAT-01: a chat turns MCP tool servers on (Tools menu next to the effort chip); new chats keep the pick.
test("the Tools menu turns tool servers on for a new chat and keeps them after the first message", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('#left .new-chat');
  const chip = page.locator(".tools-chip");
  await expect(chip).toHaveAttribute("aria-label", /^No tools/);
  await chip.click();
  const items = page.locator('.tools-menu [role="menuitemcheckbox"]');
  await expect(items.locator(".effort-name")).toHaveText(["Stack Overflow", "Developer docs", "Past chats", "Remember facts"]);
  await items.first().click();
  await expect(items.first().locator(".effort-hint")).toHaveText("3.5 million answered programming questions");
  await expect(items.first()).toHaveAttribute("aria-checked", "true");
  await expect(chip).toHaveText("1");
  await page.keyboard.press("Escape");
  await expect(page.locator(".tools-menu")).toHaveCount(0);
  // UI-02: a click outside closes it, like the Effort chip; Web is its own chip, not in the list.
  await chip.click();
  await page.click("#conv-head h1");
  await expect(page.locator(".tools-menu")).toHaveCount(0);
  const web = page.locator(".web-chip");
  await expect(web).toHaveAttribute("aria-pressed", "false");
  await web.click();
  await expect(web).toHaveAttribute("aria-pressed", "true");
  await expect(chip).toHaveText("1");
  await page.fill("#prompt", "How do I read a file in Rust?");
  await page.press("#prompt", "Enter");
  await expect(page.locator("#conv-head h1")).toHaveText("How do I read a file in Rust?");
  await expect(chip).toHaveText("1");
});

test("an existing chat keeps its own tools", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  const first = page.locator('#left [data-action="open-chat"]').first();
  await first.click();
  const chip = page.locator(".tools-chip");
  await expect(chip).toHaveAttribute("aria-label", /^No tools/);
  await chip.click();
  await page.locator('.tools-menu [role="menuitemcheckbox"]', { hasText: "Developer docs" }).click();
  await expect(chip).toHaveText("1");
  await page.locator('#left [data-action="open-chat"]').nth(1).click();
  await expect(chip).toHaveAttribute("aria-label", /^No tools/);
  await first.click();
  await expect(chip).toHaveText("1");
});

// HIST-01: past chats are a built-in tool, shown as "Past chats" with what they do, and turn on per chat.
test("Past chats is in the Tools menu and turns on for the chat without a reload", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.locator('#left [data-action="open-chat"]').first().click();
  const chip = page.locator(".tools-chip");
  await chip.click();
  const item = page.locator('.tools-menu [role="menuitemcheckbox"]', { hasText: "Past chats" });
  await expect(item.locator(".effort-hint")).toHaveText(/search them and read back what was said/);
  await expect(item).toHaveAttribute("aria-checked", "false");
  await item.click();
  await expect(item).toHaveAttribute("aria-checked", "true");
  await expect(chip).toHaveText("1");
  await expect(chip).toHaveAttribute("aria-label", /^1 tool on/);
});

// HIST-01: at phone width the Tools menu stays on screen (it used to run off the left edge), in both themes.
for (const scheme of ["light", "dark"] as const) {
  test(`the Tools menu fits a 390 px phone (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto("/?demo");
    await page.click("button.found-server");
    await page.locator('#left [data-action="open-chat"]').first().click();
    await page.setViewportSize({ width: 390, height: 844 });
    await page.addStyleTag({ content: "body.demo::before{display:none!important}" });
    await page.locator(".tools-chip").click();
    const box = (await page.locator(".tools-menu").boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(390);
    // Wide enough to read: it spans the hint line, not the chip.
    expect(box.width).toBeGreaterThan(300);
    const item = page.locator('.tools-menu [role="menuitemcheckbox"]', { hasText: "Past chats" });
    expect((await item.boundingBox())!.height).toBeGreaterThanOrEqual(44);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
  });
}
