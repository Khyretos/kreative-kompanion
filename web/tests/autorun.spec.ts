import { test, expect, type Page } from "@playwright/test";

async function openChat(page: Page): Promise<void> {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await expect(page.locator("#left")).toBeVisible();
  await page.click("#left .new-chat");
}

test.describe("Automatic mode chip (AUTO-01)", () => {
  test("chip toggles before the chat exists and the choice survives the first message", async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await openChat(page);
    const chip = page.locator("#autorun-slot .autorun-chip");
    await expect(chip).toHaveAttribute("aria-pressed", "false");
    await expect(chip).toHaveAttribute("title", /I ask before each step/);
    await chip.click();
    await expect(chip).toHaveAttribute("aria-pressed", "true");
    await expect(chip).toHaveAttribute("title", /up to 10 steps per request/);

    await page.fill("#prompt", "Run this on your own");
    await page.press("#prompt", "Enter");
    await expect(page.locator("#conv-head .conv-title h1")).toHaveText("Run this on your own");
    // The draft became the chat's own setting.
    await expect(chip).toHaveAttribute("aria-pressed", "true");

    await chip.click();
    await expect(chip).toHaveAttribute("aria-pressed", "false");

    // A new chat starts with it off again.
    await page.click("#left .new-chat");
    await expect(page.locator("#autorun-slot .autorun-chip")).toHaveAttribute("aria-pressed", "false");
    expect(errors).toEqual([]);
  });

  test("a project switch locks it on for every chat of that project", async ({ page }) => {
    await page.goto("/?demo");
    await page.click("button.found-server");
    await page.click('[data-action="project"][data-id="p-kk"]');
    await page.click('#right .ov-sec[data-sec="settings"] [data-action="overview-sec"]');
    const box = page.locator("#project-autorun");
    await expect(box).not.toBeChecked();
    await box.check();
    await page.click('[data-action="new-chat"][data-project="p-kk"]');
    const chip = page.locator("#autorun-slot .autorun-chip");
    await expect(chip).toHaveAttribute("aria-pressed", "true");
    await expect(chip).toHaveAttribute("aria-disabled", "true");
    await expect(chip).toHaveAttribute("title", /without asking in this project/);
    await chip.click({ force: true }); // locked: nothing changes
    await expect(chip).toHaveAttribute("aria-pressed", "true");
    // A loose chat is not affected.
    await page.click("#left .new-chat");
    await expect(page.locator("#autorun-slot .autorun-chip")).toHaveAttribute("aria-pressed", "false");
  });

  test("same compact size as the other chips, also at phone width", async ({ page }) => {
    await openChat(page);
    const chip = page.locator("#autorun-slot .autorun-chip");
    await expect(chip).toBeVisible();
    await expect.poll(() => chip.evaluate((el) => el.getBoundingClientRect().height)).toBe(26);
    await page.setViewportSize({ width: 390, height: 844 });
    await expect(chip).toBeVisible();
    await expect.poll(() => chip.evaluate((el) => el.getBoundingClientRect().height)).toBe(26);
  });
});
