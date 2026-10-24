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

  // EFF-02: the chip shows only the effort; the model sits on top of the picker and changes there.
  test("the chip shows only the effort; the picker names the model and changes it live", async ({ page }) => {
    const chip = page.locator("button.effort-chip");
    await expect(chip).toHaveText("Auto ▾");
    await expect(chip).toHaveAttribute("aria-label", "Effort Auto, model deepseek-chat. Change effort or model");
    await chip.click();
    const head = page.locator(".effort-menu .effort-model");
    await expect(head.locator(".effort-name")).toHaveText("deepseek-chat");
    await head.click();
    const picks = page.locator(".effort-models [role='menuitemradio']");
    await expect(picks).toHaveCount(3);
    await expect(picks.filter({ hasText: "deepseek-chat" })).toHaveAttribute("aria-checked", "true");
    await expect(picks.filter({ hasText: "deepseek-chat" })).toBeFocused();
    await page.locator(".effort-models").getByText("claude-sonnet-5-5").click();
    // The list folds, the menu stays open with the new model, and nothing reloaded.
    await expect(page.locator(".effort-models")).toHaveCount(0);
    await expect(head.locator(".effort-name")).toHaveText("claude-sonnet-5-5");
    await expect(chip).toHaveText("Auto ▾");
    await expect(chip).toHaveAttribute("title", "claude-sonnet-5-5");
    // Picking an effort still works below it.
    await page.click('[data-effort="high"]');
    await expect(chip).toHaveText("High ▾");
  });

  for (const colorScheme of ["dark", "light"] as const) {
    for (const width of [1280, 390]) {
      test(`the open picker fits and reads well at ${width} px (${colorScheme})`, async ({ page }) => {
        await page.emulateMedia({ colorScheme });
        await page.goto("/?demo");
        await page.click("button.found-server");
        // As in the phone test above: load wide, then narrow (a phone load opens the sidebar first).
        await page.setViewportSize({ width, height: 844 });
        // The demo banner covers the composer on a phone; wait for the side panel to slide away.
        await page.addStyleTag({ content: "body.demo::before{display:none!important}" });
        await page.waitForFunction(() => {
          const chip = document.querySelector(".effort-chip");
          return !!chip && document.getAnimations().length === 0 && chip.getBoundingClientRect().x >= 0;
        });
        await page.click("button.effort-chip");
        await page.click(".effort-menu .effort-model");
        // The list is drawn on the next frame: measure that menu, not the one it replaces.
        await expect(page.locator(".effort-models [aria-checked='true']")).toBeFocused();
        const menu = page.locator(".effort-menu");
        const box = (await menu.boundingBox())!;
        expect(box.x).toBeGreaterThanOrEqual(0);
        expect(box.x + box.width).toBeLessThanOrEqual(width);
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1)).toBe(true);
        const ratio = await page.locator(".effort-change").evaluate((el) => {
          const rgb = (c: string) => (c.match(/[\d.]+/g) ?? []).map(Number);
          const lum = ([r, g, b]: number[]) => {
            const f = (v: number) => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; };
            return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
          };
          let bg: number[] = [];
          for (let n: Element | null = el; n && !bg.length; n = n.parentElement) {
            const c = rgb(getComputedStyle(n).backgroundColor);
            if (c.length === 3 || (c.length === 4 && c[3] === 1)) bg = c.slice(0, 3);
          }
          const [a, b] = [lum(rgb(getComputedStyle(el).color)), lum(bg.length ? bg : [255, 255, 255])].sort((x, y) => y - x);
          return (a + 0.05) / (b + 0.05);
        });
        expect(ratio).toBeGreaterThanOrEqual(4.5);
        await menu.screenshot({ path: `test-results/effort-picker-${width}-${colorScheme}.png` });
      });
    }
  }
});
