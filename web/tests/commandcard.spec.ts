import { test, expect, type Page } from "@playwright/test";

// CMD-01: steps are cards (type, time, risk, state; the command as code) with "Explain command".
async function cards(page: Page, scheme: "light" | "dark" = "dark"): Promise<void> {
  await page.emulateMedia({ colorScheme: scheme });
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.selectOption("#pc-machine", { label: "soucouyant" });
  await page.fill("#prompt", "show me command cards");
  await page.press("#prompt", "Enter");
  await expect(page.locator("#messages details.step-card")).toHaveCount(4);
  // The demo reply streams in (and redraws its steps with every piece): wait until it is written.
  await expect(page.locator("#msg-list .caret")).toHaveCount(0, { timeout: 15_000 });
}

async function explainOn(page: Page, verbose = false): Promise<void> {
  await page.locator('[data-action="settings"]').first().click();
  await page.check("#explain-on");
  await expect(page.locator("#explain-model")).toBeEnabled();
  await expect(page.locator(".explain-settings")).toContainText("Uses extra tokens each time you open an explanation.");
  if (verbose) await page.check('input[name="explain-mode"][value="verbose"]');
  await page.keyboard.press("Escape"); // back to the chat
}

test("a step card shows type, time, risk and state, and the command as highlighted code", async ({ page }) => {
  await cards(page);
  const card = (state: string, nth = 0) => page.locator(`#messages details.step-card[data-state="${state}"]`).nth(nth);
  const grep = card("done", 0);
  await expect(grep.locator(".step-type .kind-tag")).toHaveText("Run command");
  await expect(grep.locator(".step-time")).toHaveText("0:02");
  await expect(grep.locator(".risk-tag")).toHaveText("read only");
  await expect(grep.locator("summary .chip")).toHaveText("done");
  await expect(card("done", 1).locator(".risk-tag")).toHaveText("deletes");
  await expect(card("failed").locator(".risk-tag")).toHaveText("network");
  // The running one is open, ticks, and shows its command highlighted and scrollable.
  const run = card("running");
  await expect(run).toHaveAttribute("open", "");
  await expect(run.locator(".step-time .elapsed")).toHaveText(/^0:1\d$/);
  await expect(run.locator(".risk-tag")).toHaveText("changes files");
  const code = run.locator(".step-code code");
  await expect(code).toContainText("cmake --build build -j && ctest");
  await expect(code.locator("span[class^='hljs-']").first()).toBeVisible();
  await expect.poll(() => run.locator(".step-code").evaluate((el) => getComputedStyle(el).overflowY)).toBe("auto");
  // Explanations are off by default: no footer.
  await expect(page.locator(".step-explain")).toHaveCount(0);
});

test("Explain command appears with the setting, is written on the first open and kept", async ({ page }) => {
  await cards(page);
  await explainOn(page);
  const grep = page.locator('#messages details.step-card[data-state="done"]').first();
  await grep.locator("summary").first().click();
  const explain = grep.locator(".step-explain");
  await expect(explain).not.toHaveAttribute("open", "");
  await explain.locator("summary").click();
  const body = explain.locator(".step-explain-body");
  await expect(body).toContainText("Explaining…");
  await expect(body.locator("strong")).toHaveText("changes nothing");
  // Closed and opened again: the same text at once, no new request.
  await explain.locator("summary").click();
  await explain.locator("summary").click();
  await expect(body.locator("strong")).toHaveText("changes nothing");
  // It stays open across a live re-render of the chat.
  await page.fill("#prompt", "thanks");
  await page.press("#prompt", "Enter");
  await expect(page.locator("#messages .step-explain[open] .step-explain-body strong").first()).toHaveText("changes nothing");
});

test("turning the setting off removes the footer at once", async ({ page }) => {
  await cards(page);
  await explainOn(page);
  await expect(page.locator(".step-explain")).toHaveCount(4);
  await page.locator('[data-action="settings"]').first().click();
  await page.uncheck("#explain-on");
  await expect(page.locator("#explain-model")).toBeDisabled();
  await page.keyboard.press("Escape"); // back to the chat
  await expect(page.locator(".step-explain")).toHaveCount(0);
});

for (const scheme of ["light", "dark"] as const) {
  for (const width of [1280, 390]) {
    test(`cards fit and read well at ${width} px (${scheme})`, async ({ page }) => {
      await cards(page, scheme);
      await explainOn(page);
      await page.setViewportSize({ width, height: 900 });
      await page.addStyleTag({ content: "body.demo::before{display:none!important}" });
      const run = page.locator('#messages details.step-card[data-state="running"]');
      await run.locator(".step-explain summary").click();
      await expect(run.locator(".step-explain-body strong")).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1)).toBe(true);
      const ratios = await page.locator("#messages .risk-tag, #messages .step-time, #messages .step-explain > summary").evaluateAll((els) => els.filter((e) => (e as HTMLElement).offsetParent).map((el) => {
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
        return { text: el.textContent?.trim(), ratio: (a + 0.05) / (b + 0.05) };
      }));
      for (const r of ratios) expect(r.ratio, r.text).toBeGreaterThanOrEqual(4.5);
      // A live update can redraw the card mid-shot: take it again until it holds still.
      await expect(async () => { await run.screenshot({ path: `test-results/commandcard-${width}-${scheme}.png`, timeout: 2000 }); }).toPass({ timeout: 10_000 });
    });
  }
}
