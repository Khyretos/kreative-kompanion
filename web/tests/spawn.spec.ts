import { test, expect, type Page } from "@playwright/test";

// AGENT-SPAWN: the model asks to start a new chat; the user approves it like any step.
async function ask(page: Page, scheme: "light" | "dark" = "dark"): Promise<void> {
  await page.emulateMedia({ colorScheme: scheme });
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.selectOption("#pc-machine", { label: "soucouyant" });
  await page.fill("#prompt", "hand this on to a new chat");
  await page.press("#prompt", "Enter");
}

test("the approval card shows the new chat's prompt; Approve makes the chat, which opens from the step", async ({ page }) => {
  await ask(page);
  const card = page.locator(".pc-action.spawn");
  await expect(card.locator(".card-kind")).toContainText("New chat");
  await expect(card.locator("strong")).toHaveText('Start a new chat: "Logo ideas for the shop"');
  await expect(card.locator(".spawn-prompt")).toContainText("Sketch three logo ideas");
  await expect(card.locator('[data-decision="always"]')).toHaveCount(0);
  await card.locator('[data-decision="approve"]').click();
  // Done, it moves into the chat as a step card with an Open button; nothing reloaded.
  const step = page.locator('#messages details.step-card[data-state="done"]').last();
  await expect(step.locator(".kind-tag")).toHaveText("New chat");
  await expect(step.locator(".risk-tag")).toHaveCount(0);
  await expect(page.locator("#left")).toContainText("Logo ideas for the shop");
  await step.locator("summary").first().click();
  await step.locator('[data-action="open-chat"]').click();
  await expect(page.locator(".conv-head")).toContainText("Logo ideas for the shop");
  await expect(page.locator("#msg-list .msg.user").first()).toContainText("Sketch three logo ideas");
});

test("Deny makes nothing", async ({ page }) => {
  await ask(page);
  const before = await page.locator("#left [data-action=\"open-chat\"]").count();
  await page.locator('.pc-action.spawn [data-decision="deny"]').click();
  await expect(page.locator('#messages details.step-card[data-state="denied"]').last().locator(".kind-tag")).toHaveText("New chat");
  await expect(page.locator("#left [data-action=\"open-chat\"]")).toHaveCount(before);
});

for (const scheme of ["light", "dark"] as const) {
  for (const width of [1280, 390]) {
    test(`the approval card fits and reads well at ${width} px (${scheme})`, async ({ page }) => {
      await ask(page, scheme);
      await page.setViewportSize({ width, height: 900 });
      await page.addStyleTag({ content: "body.demo::before{display:none!important}" });
      const card = page.locator(".pc-action.spawn");
      await expect(card).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1)).toBe(true);
      const box = (await card.boundingBox())!;
      expect(box.x + box.width).toBeLessThanOrEqual(width);
      const ratio = await card.locator(".spawn-prompt").evaluate((el) => {
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
      expect(ratio).toBeGreaterThanOrEqual(7);
      await expect(async () => { await card.screenshot({ path: `test-results/spawn-${width}-${scheme}.png`, timeout: 2000 }); }).toPass({ timeout: 10_000 });
    });
  }
}

test("a task the model asks for lands in the project's task list once approved", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.selectOption("#pc-machine", { label: "soucouyant" });
  await page.fill("#prompt", "hand this on as a task");
  await page.press("#prompt", "Enter");
  const card = page.locator(".pc-action.spawn");
  await expect(card.locator(".card-kind")).toContainText("New task");
  await card.locator('[data-decision="approve"]').click();
  const step = page.locator('#messages details.step-card[data-state="done"]').last();
  await expect(step.locator(".kind-tag")).toHaveText("New task");
  await step.locator("summary").first().click();
  await step.locator('[data-action="open-task"]').click();
  await expect(page.locator("#right")).toContainText("Write the battle rules");
});
