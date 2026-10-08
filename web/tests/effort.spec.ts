import { test, expect } from "@playwright/test";

test.describe("Effort chip", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/?demo");
    await page.click("button.found-server");
  });

  test("shows Auto by default and opens a menu with four levels", async ({ page }) => {
    const chip = page.locator("button.effort-chip");
    await expect(chip).toHaveText(/Auto ▾$/);

    await chip.click();
    const menu = page.locator(".effort-menu");
    await expect(menu).toBeVisible();

    const items = page.locator("button[role='menuitemradio']");
    await expect(items).toHaveCount(4);

    const autoItem = items.filter({ hasText: "Auto" }).first();
    await expect(autoItem).toHaveAttribute("aria-checked", "true");
    await expect(autoItem).toBeFocused();
  });

  test("picking High updates the chip without reload and keeps it for the new chat", async ({ page }) => {
    const chip = page.locator("button.effort-chip");
    await chip.click();

    const highItem = page.locator("button[role='menuitemradio']").filter({ hasText: "High" });
    await highItem.click();

    const menu = page.locator(".effort-menu");
    await expect(menu).toHaveCount(0);

    await expect(chip).toHaveText(/High ▾$/);
    await expect(chip).toBeFocused();

    await page.fill("#prompt", "hello");
    await page.press("#prompt", "Enter");

    await page.waitForSelector("#msg-list .msg", { state: "visible" });

    await expect(chip).toHaveText(/High ▾$/);
  });

  test("Escape closes the menu and the chip stays compact on phones", async ({ page }) => {
    const chip = page.locator("button.effort-chip");
    await chip.click();

    await page.keyboard.press("Escape");
    const menu = page.locator(".effort-menu");
    await expect(menu).toHaveCount(0);
    await expect(chip).toBeFocused();

    await page.setViewportSize({ width: 375, height: 800 });
    await page.waitForTimeout(100);
    const box = await chip.boundingBox();
    // UI-04: chips are compact (26px) on phones too; only icon-only buttons keep the big touch target.
    expect(box?.height).toBeLessThanOrEqual(28);
  });

  test("the chip text reaches 7:1 contrast in dark and light", async ({ page }) => {
  for (const colorScheme of ["dark", "light"] as const) {
    await page.emulateMedia({ colorScheme });
    await page.goto("/?demo");
    await page.click("button.found-server");

    const chip = page.locator("button.effort-chip");
    await expect(chip).toBeVisible();

    // Wait for styles to settle after navigation
    await page.waitForTimeout(100);

    const contrast = await chip.evaluate((el) => {
      const parse = (s: string) => s.replace(/rgba?\(/, "").replace(")", "").split(",").map(Number);
      const lum = (s: string) => {
        const [r, g, b] = parse(s).map((c) => { const v = c / 255; return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); });
        return 0.2126 * r + 0.7152 * g + 0.0722 * b;
      };
      const st = getComputedStyle(el);
      const [l1, l2] = [lum(st.color), lum(st.backgroundColor)].sort((x, y) => y - x);
      return (l1 + 0.05) / (l2 + 0.05);
    });

    expect(contrast, colorScheme).toBeGreaterThanOrEqual(7);
  }
  });

  test("in an existing chat the level is stored on the chat", async ({ page }) => {
    await page.fill("#prompt", "hello");
    await page.press("#prompt", "Enter");
    await expect(page.locator("#msg-list .msg").first()).toBeVisible();
    const chip = page.locator("button.effort-chip");
    await chip.click();
    await page.click('[data-effort="low"]');
    await expect(chip).toHaveText(/Low ▾$/);
    await expect(chip).toBeFocused();
    await page.click('[data-action="new-chat"]');
    await expect(chip).toHaveText(/Auto ▾$/);
  });
});
