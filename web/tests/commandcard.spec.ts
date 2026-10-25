import { test, expect, type Page } from "@playwright/test";

// CMD-01 / COMPACT-UI: a step is a compact card: one header row (icon, type, time; Explain, risk,
// state), then the command as bash-highlighted code, three lines until opened.
async function cards(page: Page, scheme: "light" | "dark" = "dark", width = 1280): Promise<void> {
  await page.emulateMedia({ colorScheme: scheme });
  await page.setViewportSize({ width, height: 900 });
  await page.goto("/?demo");
  await page.click("button.found-server");
  if (width < 768) {
    await page.addStyleTag({ content: "body.demo::before{display:none!important}" });
    await page.waitForFunction(() => document.getAnimations().filter((a) => !(a as CSSAnimation).animationName?.startsWith("tov-pulse")).length === 0);
  }
  await page.selectOption("#pc-machine", { label: "soucouyant" });
  await page.fill("#prompt", "show me command cards");
  await page.press("#prompt", "Enter");
  await expect(page.locator("#messages details.step-card")).toHaveCount(4);
  // The demo reply streams in (and redraws its steps with every piece): wait until it is written.
  await expect(page.locator("#msg-list .caret")).toHaveCount(0, { timeout: 15_000 });
}

const card = (page: Page, state: string, nth = 0) => page.locator(`#messages details.step-card[data-state="${state}"]`).nth(nth);

test("one header row (type and time together, risk and state on the right) and the command highlighted as bash", async ({ page }) => {
  await cards(page);
  const grep = card(page, "done", 0);
  const head = grep.locator(".step-head");
  await expect(head.locator(".kind-tag")).toHaveText("Run command");
  await expect(head.locator(".step-time")).toHaveText("0:02");
  await expect(head.locator(".risk-tag")).toHaveText("read only");
  await expect(head.locator(".chip.done")).toHaveText("done");
  await expect(head.locator(".step-explain-link")).toHaveText("Explain");
  // All of it on one line.
  const tops = await head.locator(".kind-tag, .step-time, .risk-tag, .chip.done, .step-explain-link").evaluateAll((els) => els.map((e) => Math.round(e.getBoundingClientRect().top)));
  expect(Math.max(...tops) - Math.min(...tops)).toBeLessThan(6);
  // Closed, the command still shows: bash colours, the folder as a short prompt, no plain summary line.
  await expect(grep).not.toHaveAttribute("open", "");
  const line = grep.locator(".step-line");
  await expect(line).toContainText('grep -rn "TODO" src | head -20');
  await expect(line.locator(".hljs-built_in, .hljs-string").first()).toBeVisible();
  await expect(line.locator(".step-cwd")).toHaveText("…/kees/projects/kk-engine $");
  await expect(grep.locator(".step-summary")).toHaveCount(0);
  await expect(card(page, "done", 1).locator(".risk-tag")).toHaveText("deletes");
  await expect(card(page, "failed").locator(".risk-tag")).toHaveText("network");
  // The running one is open and ticks; open, the command scrolls instead of being cut.
  const run = card(page, "running");
  await expect(run).toHaveAttribute("open", "");
  await expect(run.locator(".step-time.elapsed")).toHaveText(/^0:1\d$/);
  await expect(run.locator(".risk-tag")).toHaveText("changes files");
  await expect(run.locator(".step-line")).toContainText("cmake --build build -j && ctest");
});

test("Explain is on every command card with no setting; the first click writes it, later clicks read it back", async ({ page }) => {
  await cards(page);
  await expect(page.locator("#messages .step-explain-link")).toHaveCount(4);
  const grep = card(page, "done", 0);
  const link = grep.locator(".step-explain-link");
  await link.click();
  await expect(grep).toHaveAttribute("open", "");
  await expect(link).toHaveAttribute("aria-expanded", "true");
  const body = grep.locator(".step-explain-body");
  await expect(body.locator("strong")).toHaveText("changes nothing");
  await link.click();
  await expect(body).toBeHidden();
  await link.click();
  await expect(body.locator("strong")).toHaveText("changes nothing");
  // It stays open across a live re-render of the chat.
  await page.fill("#prompt", "thanks");
  await page.press("#prompt", "Enter");
  await expect(page.locator("#messages .step-explain-body:not([hidden]) strong").first()).toHaveText("changes nothing");
});

test("Settings only picks the detail and the model, and says what a click costs", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.locator('[data-action="settings"]').first().click();
  const sec = page.locator(".explain-settings");
  await expect(sec.locator("#explain-on")).toHaveCount(0);
  await expect(sec.locator('input[name="explain-mode"][value="compact"]')).toBeChecked();
  await expect(sec.locator("#explain-model")).toBeEnabled();
  await expect(sec).toContainText("Each first click on Explain uses some tokens");
});

test("a long path shows its last parts, the full one on hover", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.selectOption("#pc-machine", { label: "soucouyant" });
  await page.fill("#prompt", "show me every card type");
  await page.press("#prompt", "Enter");
  await page.locator('.pc-action [data-decision="approve"]').first().click();
  const path = page.locator("#messages details.step-card .step-path").first();
  await expect(path).toHaveText("…/projects/kk-engine/Cargo.toml");
  await expect(path).toHaveAttribute("title", "/home/kees/projects/kk-engine/Cargo.toml");
});

for (const scheme of ["light", "dark"] as const) {
  for (const width of [1280, 390]) {
    test(`cards fit and read well at ${width} px (${scheme})`, async ({ page }) => {
      await cards(page, scheme, width);
      const run = card(page, "running");
      await run.locator(".step-explain-link").click();
      await expect(run.locator(".step-explain-body strong")).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1)).toBe(true);
      // The header stays one row at phone width too.
      const done = card(page, "done", 0).locator(".step-head");
      await expect.poll(async () => (await done.boundingBox())?.height ?? 99).toBeLessThan(40); // polled: a live redraw can swap the card
      const ratios = () => page.locator("#messages .risk-tag, #messages .step-time, #messages .step-explain-link, #messages .step-line").evaluateAll((els) => els.filter((e) => (e as HTMLElement).offsetParent).map((el) => {
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
    });
  }
}
