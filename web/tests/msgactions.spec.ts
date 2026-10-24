import { test, expect, type Page } from "@playwright/test";

// MSG-ACT: Copy every message, Regenerate the newest answer, Edit and resend your own message.
async function chat(page: Page, scheme: "light" | "dark" = "dark"): Promise<void> {
  await page.emulateMedia({ colorScheme: scheme });
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="new-chat"]');
  await page.fill("#prompt", "hello there");
  await page.press("#prompt", "Enter");
  await expect(page.locator("#msg-list .msg.orchestrator")).toHaveCount(1);
  await expect(page.locator("#msg-list .caret")).toHaveCount(0, { timeout: 15_000 });
}

test("Copy puts the message's own words on the clipboard", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await chat(page);
  const answer = page.locator("#msg-list .msg.orchestrator").last();
  await answer.hover();
  await answer.locator('[data-action="msg-copy"]').click();
  await expect(answer.locator('[data-action="msg-copy"]')).toHaveClass(/done/);
  const copied = await page.evaluate(() => navigator.clipboard.readText());
  expect(copied.length).toBeGreaterThan(20);
  expect(await answer.locator(".body").innerText()).toContain(copied.split("\n")[0].replace(/\*\*/g, ""));
  await page.locator("#msg-list .msg.user").last().locator('[data-action="msg-copy"]').click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe("hello there");
});

test("Regenerate is on the newest answer only and replaces it without a reload", async ({ page }) => {
  await chat(page);
  await page.fill("#prompt", "and another");
  await page.press("#prompt", "Enter");
  await expect(page.locator("#msg-list .msg.orchestrator")).toHaveCount(2);
  await expect(page.locator("#msg-list .caret")).toHaveCount(0, { timeout: 15_000 });
  await expect(page.locator('#msg-list [data-action="msg-regen"]')).toHaveCount(1);
  const last = page.locator("#msg-list .msg.orchestrator").last();
  const oldId = await last.getAttribute("id");
  await last.locator('[data-action="msg-regen"]').click();
  await expect(page.locator(`#${oldId}`)).toHaveCount(0);
  await expect(page.locator("#msg-list .msg.orchestrator")).toHaveCount(2);
  await expect(page.locator("#msg-list .caret")).toHaveCount(0, { timeout: 15_000 });
  await expect(page.locator("#msg-list .msg").last()).toHaveClass(/orchestrator/);
});

test("Edit and resend changes your message, drops what followed and answers again; Escape cancels", async ({ page }) => {
  await chat(page);
  const mine = page.locator("#msg-list .msg.user").first();
  await mine.locator('[data-action="msg-edit"]').click();
  const box = mine.locator(".msg-edit-text");
  await expect(box).toBeFocused();
  await expect(box).toHaveValue("hello there");
  await page.keyboard.press("Escape");
  await expect(mine.locator(".body")).toHaveText("hello there");
  await expect(mine.locator('[data-action="msg-edit"]')).toBeFocused();
  await mine.locator('[data-action="msg-edit"]').click();
  await box.fill("hello again, edited");
  await box.press("Enter");
  await expect(mine.locator(".body")).toHaveText("hello again, edited");
  await expect(page.locator("#msg-list .msg")).toHaveCount(2);
  await expect(page.locator("#msg-list .caret")).toHaveCount(0, { timeout: 15_000 });
  await expect(page.locator("#msg-list .msg").last()).toHaveClass(/orchestrator/);
});

for (const scheme of ["light", "dark"] as const) {
  for (const width of [1280, 390]) {
    test(`message actions fit and read well at ${width} px (${scheme})`, async ({ page }) => {
      await chat(page, scheme);
      await page.setViewportSize({ width, height: 900 });
      const answer = page.locator("#msg-list .msg.orchestrator").last();
      await answer.hover();
      const act = answer.locator('[data-action="msg-regen"]');
      await expect(act).toBeVisible();
      await expect.poll(() => act.evaluate((el) => getComputedStyle(el.parentElement!).opacity)).toBe("1");
      if (width < 768) expect((await act.boundingBox())!.height).toBeGreaterThanOrEqual(44);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1)).toBe(true);
      const ratio = await act.evaluate((el) => {
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
      await page.locator("#msg-list .msg.user").last().locator('[data-action="msg-edit"]').click();
      await expect(async () => { await page.locator("#msg-list").screenshot({ path: `test-results/msgactions-${width}-${scheme}.png`, timeout: 2000 }); }).toPass({ timeout: 10_000 });
    });
  }
}
