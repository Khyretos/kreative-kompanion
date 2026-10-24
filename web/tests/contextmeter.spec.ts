import { test, expect, type Page } from "@playwright/test";

// CTX-01: the context meter in the composer: tokens used vs the model's window, a warning near the
// limit, the summary of the older messages and "Summarise now", all without a reload.
async function openChat(page: Page, title: string) {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.locator('#left [data-action="open-chat"]', { hasText: title }).click();
}

test("a nearly full chat shows the meter in orange, its numbers, the summary and the warning", async ({ page }) => {
  await openChat(page, "Debug tooling and tests");
  const chip = page.locator(".ctx-chip");
  await expect(chip).toHaveText("104k/128k");
  await expect(chip).toHaveAttribute("data-level", "warn");
  await expect(chip).toHaveAttribute("aria-label", /^Context 81% full: 104,300 of 128,000 tokens/);
  await chip.click();
  const pop = page.locator(".ctx-pop");
  await expect(pop.locator(".ctx-head")).toHaveText("104,300 of 128,000 tokens (81%)");
  await expect(pop.locator(".ctx-model")).toHaveText("deepseek-chat, latest answer");
  await expect(pop.locator(".ctx-warn")).toHaveText(/Nearly full: the next answer first summarises/);
  await pop.locator("summary").click();
  await expect(pop.locator("summary")).toHaveText(/^Summary of 14 older messages · /);
  await expect(pop.locator(".ctx-sum-text")).toContainText("--vk-validation flag");
  await page.keyboard.press("Escape");
  await expect(pop).toHaveCount(0);
  await expect(chip).toBeFocused();
  // A click outside closes it too.
  await chip.click();
  await page.click("#conv-head h1");
  await expect(page.locator(".ctx-pop")).toHaveCount(0);
});

test("Summarise now shrinks the context and adds to the summary without a reload", async ({ page }) => {
  await openChat(page, "Debug tooling and tests");
  await page.locator(".ctx-chip").click();
  const btn = page.locator('.ctx-pop [data-action="context-compact"]');
  await btn.click();
  await expect(btn).toHaveText("Summarising…");
  await expect(btn).toBeDisabled();
  await expect(page.locator(".ctx-chip")).toHaveText("47k/128k");
  await expect(page.locator(".ctx-chip")).toHaveAttribute("data-level", "ok");
  await expect(page.locator(".ctx-pop summary")).toHaveText(/^Summary of 1[5-9] older messages|^Summary of [2-9]\d older messages/);
  await expect(page.locator(".ctx-pop .ctx-warn")).toHaveCount(0);
  await expect(btn).toHaveText("Summarise now");
});

test("a chat without an answer has no meter until the first answer, then it appears live", async ({ page }) => {
  await openChat(page, "Which local model next?");
  await expect(page.locator(".ctx-chip")).toHaveCount(0);
  await page.fill("#prompt", "Which one fits 8 GB?");
  await page.press("#prompt", "Enter");
  const chip = page.locator(".ctx-chip");
  await expect(chip).toHaveText(/2\.\dk\/128k/, { timeout: 15_000 });
  await expect(chip).toHaveAttribute("data-level", "ok");
  // Another chat shows its own numbers, and coming back shows these again.
  await page.locator('#left [data-action="open-chat"]', { hasText: "Renderer performance" }).click();
  await expect(chip).toHaveText("18k/128k");
});

for (const scheme of ["light", "dark"] as const) {
  test(`at 390 px the meter and its details fit the phone (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await openChat(page, "Debug tooling and tests");
    await page.setViewportSize({ width: 390, height: 844 });
    await page.addStyleTag({ content: "body.demo::before{display:none!important}" });
    // Wait for the phone layout to settle (the side panel slides away).
    await page.waitForFunction(() => document.getAnimations().length === 0 && document.querySelector(".composer-hint")!.getBoundingClientRect().x >= 0);
    const chip = page.locator(".ctx-chip");
    await expect(chip).toBeVisible();
    expect((await chip.boundingBox())!.height).toBeGreaterThanOrEqual(44);
    await chip.click();
    const box = (await page.locator(".ctx-pop").boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(390);
    expect(box.width).toBeGreaterThan(300);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
    // The warning text keeps AA contrast on its tinted background.
    const ratio = await page.locator(".ctx-warn").evaluate((el) => {
      const rgb = (s: string) => (s.match(/[\d.]+/g) ?? []).slice(0, 3).map(Number);
      const lum = (c: number[]) => {
        const [r, g, b] = c.map((v) => { const x = v / 255; return x <= 0.03928 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4; });
        return 0.2126 * r + 0.7152 * g + 0.0722 * b;
      };
      // The tint is see-through: blend it over the popover's own background.
      const pop = getComputedStyle(el.closest(".ctx-pop")!).backgroundColor;
      const tint = getComputedStyle(el).backgroundColor;
      const a = Number((tint.match(/[\d.]+/g) ?? [])[3] ?? 1);
      const bg = rgb(tint).map((v, i) => v * a + rgb(pop)[i] * (1 - a));
      const [l1, l2] = [lum(rgb(getComputedStyle(el).color)), lum(bg)].sort((x, y) => y - x);
      return (l1 + 0.05) / (l2 + 0.05);
    });
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });
}
