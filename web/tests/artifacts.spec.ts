import { test, expect, type Page } from "@playwright/test";

// ARTIFACTS: long code and HTML open whole in a side pane (a sheet on phones); HTML runs in a
// sandboxed frame with no network and no access to the app.
async function demo(page: Page, width = 1280, scheme: "light" | "dark" = "dark"): Promise<void> {
  await page.emulateMedia({ colorScheme: scheme });
  await page.setViewportSize({ width, height: 900 });
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.selectOption("#pc-machine", { label: "soucouyant" });
  await page.fill("#prompt", "artifact demo");
  await page.press("#prompt", "Enter");
  // The reply streams in: wait for its last block, then for the end of the stream.
  // The demo reply takes about 15 s on its own; on a loaded CI runner far longer.
  await expect(page.locator('#msg-list .codeblock[data-lang="ts"]')).toHaveCount(1, { timeout: 45_000 });
  await expect(page.locator("#msg-list .caret")).toHaveCount(0, { timeout: 45_000 });
}
const pane = (page: Page) => page.locator("#artifact-pane");
// Every test here waits for that long reply first.
test.describe.configure({ timeout: 90_000 });

test("an HTML block shows a few lines and previews in a sandbox that runs its script offline", async ({ page }) => {
  await demo(page);
  const block = page.locator('#msg-list .codeblock[data-lang="html"]');
  await expect(block).toHaveClass(/long/);
  expect((await block.locator("pre").boundingBox())!.height).toBeLessThan(200);
  const open = block.locator('[data-action="code-open"]');
  await expect(open).toHaveText("Preview");
  await open.click();
  await expect(pane(page)).toBeVisible();
  await expect(pane(page).locator(".art-title")).toHaveText("html");
  await expect(pane(page).locator('[data-art="preview"]')).toHaveAttribute("aria-selected", "true");
  const frameEl = pane(page).locator("iframe.art-frame");
  await expect(frameEl).toHaveAttribute("sandbox", "allow-scripts");
  const frame = page.frameLocator("#artifact-pane iframe.art-frame");
  await expect(frame.locator("h1")).toHaveText("Medabots");
  await frame.locator("#fight").click();
  await expect(frame.locator("#log")).toHaveText("Round 1: the left arm takes 3 damage.");
  // Opaque origin, no network: it can't reach the app's API or its cookies.
  const inner = page.frames().find((f) => f.url().endsWith("sandbox.html"))!;
  expect(await inner.evaluate(() => origin)).toBe("null");
  expect(await inner.evaluate(() => fetch("/api/status").then(() => "reached", () => "blocked"))).toBe("blocked");
  // Code shows it highlighted; Escape closes and gives focus back.
  await pane(page).locator('[data-art="code"]').click();
  await expect(pane(page).locator(".art-code .hljs-tag").first()).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(pane(page)).toHaveCount(0);
  await expect(open).toBeFocused();
});

test("a long code block opens as code; copy and download take all of it", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await demo(page);
  const block = page.locator('#msg-list .codeblock[data-lang="ts"]');
  await expect(block).toHaveClass(/long/);
  await block.locator('[data-action="code-open"]').click();
  await expect(pane(page).locator(".art-tabs")).toHaveCount(0);
  await expect(pane(page).locator(".art-meta")).toHaveText("ts · 26 lines");
  await expect(pane(page).locator(".art-code")).toContainText("export function turn(a: Bot, b: Bot");
  await pane(page).locator('[data-art="copy"]').click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toContain("export function damage(from: Part, to: Part)");
  const [download] = await Promise.all([page.waitForEvent("download"), pane(page).locator('[data-art="download"]').click()]);
  expect(download.suggestedFilename()).toBe("artifact.ts");
  await pane(page).locator('[data-art="close"]').click();
  await expect(pane(page)).toHaveCount(0);
});

test("a file the model wrote opens from its step card under its own name", async ({ page }) => {
  await demo(page);
  const step = page.locator("#messages details.step-card").filter({ hasText: "battle.ts" });
  const open = step.locator('[data-action="artifact-step"]');
  await expect(open).toHaveText("Open");
  await open.click();
  await expect(step).not.toHaveAttribute("open", ""); // the link opens the pane, not the card
  await expect(pane(page).locator(".art-title")).toHaveText("battle.ts");
  await expect(pane(page).locator(".art-title")).toHaveAttribute("title", "/home/kees/projects/medabots/src/battle.ts");
  await expect(pane(page).locator(".art-code")).toContainText("export interface Bot");
});

for (const scheme of ["light", "dark"] as const) {
  for (const width of [390, 1280]) {
    test(`the pane fits and reads well at ${width} px (${scheme})`, async ({ page }) => {
      await demo(page, width, scheme);
      // In the chat: each block's header is one row, the language as written.
      const head = page.locator('#msg-list .codeblock[data-lang="html"] .codeblock-head');
      await expect(head.locator(".codeblock-lang")).toHaveText("html");
      expect((await head.boundingBox())!.height).toBeLessThan(48);
      // A late live update can draw the answer again: find the block afresh until it holds still.
      await expect(async () => { await page.locator('#msg-list .codeblock[data-lang="ts"]').scrollIntoViewIfNeeded({ timeout: 2000 }); }).toPass({ timeout: 10_000 });
      await page.screenshot({ path: `shots-tmp/art-chat-${width < 768 ? "phone" : "desktop"}-${scheme}.png` });
      await page.locator('#msg-list .codeblock[data-lang="html"] [data-action="code-open"]').click();
      await expect(page.frameLocator("#artifact-pane iframe.art-frame").locator("h1")).toHaveText("Medabots");
      const box = (await pane(page).boundingBox())!;
      if (width < 768) {
        // A sheet from the bottom, full width, the top bar still in view.
        expect(Math.round(box.x)).toBe(0);
        expect(Math.round(box.y + box.height)).toBe(900);
        expect(box.y).toBeGreaterThan(60);
      } else {
        expect(Math.round(box.x + box.width)).toBe(1280);
        expect(box.width).toBeLessThan(700);
      }
      // One header row, nothing sideways.
      expect((await pane(page).locator(".art-head").boundingBox())!.height).toBeLessThan(60);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1)).toBe(true);
      const low = await pane(page).locator(".art-title, .art-tabs button, .art-meta").evaluateAll((els) => els.filter((e) => (e as HTMLElement).offsetParent).map((el) => {
        const rgb = (c: string) => (c.match(/[\d.]+/g) ?? []).map(Number);
        const lum = ([r, g, b]: number[]) => { const f = (v: number) => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; }; return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b); };
        let bg: number[] = [];
        for (let n: Element | null = el; n && !bg.length; n = n.parentElement) { const c = rgb(getComputedStyle(n).backgroundColor); if (c.length === 3 || (c.length === 4 && c[3] === 1)) bg = c.slice(0, 3); }
        const [a, b] = [lum(rgb(getComputedStyle(el).color)), lum(bg.length ? bg : [255, 255, 255])].sort((x, y) => y - x);
        return { t: el.textContent, r: (a + 0.05) / (b + 0.05) };
      }).filter((x) => x.r < 4.5));
      expect(low).toEqual([]);
      await page.screenshot({ path: `shots-tmp/art-${width < 768 ? "phone" : "desktop"}-${scheme}.png` });
    });
  }
}
