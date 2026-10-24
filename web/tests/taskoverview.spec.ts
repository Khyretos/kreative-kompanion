import { test, expect, type Page } from "@playwright/test";

// TASK-OVERVIEW: the top bar's running-tasks chip, its panel, and a task's live trace.
async function open(page: Page, scheme: "light" | "dark" = "dark", width = 1280): Promise<void> {
  await page.emulateMedia({ colorScheme: scheme });
  await page.setViewportSize({ width, height: 900 });
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.addStyleTag({ content: "body.demo::before{display:none!important}" });
  await expect(page.locator("#ov-chip")).toBeVisible();
}

test("the chip counts running and done tasks; the panel numbers the running ones and drills into a live trace", async ({ page }) => {
  await open(page);
  const chip = page.locator("#ov-chip");
  await expect(chip.locator(".ov-run")).toHaveText("1");
  await expect(chip).toHaveAttribute("aria-label", /^1 running, \d+ of \d+ done/);
  await chip.click();
  const panel = page.locator(".tasks-overview");
  await expect(panel.locator(".ov-counts")).toContainText("Running");
  const row = panel.locator(".ov-row").first();
  await expect(row.locator(".ov-num")).toHaveText("1/1");
  await expect(row.locator(".ov-step")).toHaveText("Building in the container");
  await row.click();
  await expect(page.locator("#ov-back")).toBeFocused();
  await expect(panel.locator(".ov-detail-head h3")).toContainText("1/1 Add a --vk-validation flag and setting");
  await expect(panel.locator(".ov-now")).toContainText("Building in the container");
  await expect(panel.locator(".trace-call")).toHaveCount(2);
  await expect(panel.locator(".ov-trace details.step-card")).toHaveCount(2);
  const call = panel.locator(".trace-call").first();
  await expect(call.locator(".trace-model")).toHaveText("Qwen3.5-9B");
  await expect(call.locator(".trace-nums")).toContainText("6200 in · 310 out");
  await call.locator("summary").first().click();
  await call.locator(".trace-body summary").first().click();
  await expect(call.locator(".trace-body pre").first()).toContainText("--vk-validation");
  // Opened parts stay open while the trace redraws live (a new step arrives).
  await page.evaluate(() => (window as unknown as { __kkDemoTraceTick?: () => void }).__kkDemoTraceTick?.());
  await page.waitForTimeout(800);
  await expect(call.locator(".trace-body pre").first()).toBeVisible();
  // The running step is a step card with its command.
  await expect(panel.locator('.ov-trace details.step-card[data-state="running"] .step-code')).toContainText("cmake --build build");
  // Escape: back to the list, then closed, focus back on the chip.
  await page.keyboard.press("Escape");
  await expect(panel.locator(".ov-row")).toHaveCount(1);
  await expect(page.locator("#ov-row-t1")).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
  await expect(chip).toBeFocused();
});

test("a task that starts shows up at once, without a reload", async ({ page }) => {
  await open(page);
  await page.locator('#left [data-action="open-chat"]').first().click();
  await page.click("#ov-chip");
  await expect(page.locator(".tasks-overview .ov-row")).toHaveCount(1);
  await page.fill("#prompt", "Profile the shader cache");
  await page.press("#prompt", "Enter");
  await expect(page.locator("#ov-chip .ov-run")).toHaveText("2", { timeout: 15_000 });
  await expect(page.locator(".tasks-overview .ov-row")).toHaveCount(2);
  await expect(page.locator(".tasks-overview .ov-num").last()).toHaveText("2/2");
});

test("on a phone, pulling the top bar down opens the panel and pushing it up closes it", async ({ page }) => {
  await open(page, "dark", 390);
  await page.waitForFunction(() => document.getAnimations().length === 0);
  const pull = async (selector: string, from: number, to: number) => {
    await page.locator(selector).first().dispatchEvent("touchstart", { touches: [{ identifier: 1, clientX: 200, clientY: from }] });
    await page.locator(selector).first().dispatchEvent("touchmove", { touches: [{ identifier: 1, clientX: 200, clientY: to }] });
    // The panel may be gone by now (it closed): the finger lifts over the top bar.
    await page.locator("#conv-head").dispatchEvent("touchend", { touches: [] });
  };
  await pull("#conv-head", 20, 120);
  await expect(page.locator(".tasks-overview")).toBeVisible();
  const box = (await page.locator(".tasks-overview").boundingBox())!;
  expect(box.x).toBe(0);
  expect(box.width).toBe(390);
  await pull(".tasks-overview .ov-head", 120, 20);
  await expect(page.locator(".tasks-overview")).toHaveCount(0);
});

for (const scheme of ["light", "dark"] as const) {
  for (const width of [1280, 390]) {
    test(`the panel and its trace fit and read well at ${width} px (${scheme})`, async ({ page }) => {
      await open(page, scheme, width);
      await page.waitForFunction(() => document.getAnimations().filter((a) => !(a as CSSAnimation).animationName?.startsWith("ov-pulse")).length === 0);
      await page.click("#ov-chip");
      await page.locator(".tasks-overview .ov-row").first().click();
      await expect(page.locator(".tasks-overview .trace-call")).toHaveCount(2);
      const panel = page.locator(".tasks-overview");
      await expect.poll(async () => { const b = await panel.boundingBox(); return b ? b.x >= 0 && b.x + b.width <= width : null; }).toBe(true);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1)).toBe(true);
      const ratios = () => panel.locator("h2, h3, dt, dd, .ov-now, .trace-model, .trace-nums, .trace-body > summary, .ov-free").evaluateAll((els) => els.filter((e) => (e as HTMLElement).offsetParent).map((el) => {
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
      }));
      await expect.poll(async () => Math.min(...(await ratios()))).toBeGreaterThanOrEqual(4.5);
      await expect(async () => { await panel.screenshot({ path: `test-results/taskoverview-${width}-${scheme}.png`, timeout: 2000 }); }).toPass({ timeout: 10_000 });
    });
  }
}
