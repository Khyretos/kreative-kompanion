import { test, expect, type Page } from "@playwright/test";

// NOTIFY: which events, which channels, quiet hours, push in this browser, mail set by the server.
async function settings(page: Page, scheme: "light" | "dark" = "dark", width = 1280): Promise<void> {
  await page.emulateMedia({ colorScheme: scheme });
  await page.setViewportSize({ width, height: 900 });
  await page.goto("/?demo");
  await page.click("button.found-server");
  if (width < 768) await page.locator('[data-action="pane"][data-pane="left"]').first().click().catch(() => undefined);
  await page.locator('[data-action="settings"]').first().click();
  await expect(page.locator("#notify-form")).toBeVisible();
}

test("approvals, channels and quiet hours are saved and shown again", async ({ page }) => {
  await settings(page);
  const form = page.locator("#notify-form");
  await expect(form.locator('input[name="onApproval"]')).toBeChecked();
  await expect(form.locator('input[name="mailOn"]')).toBeChecked();
  await expect(form.locator('input[name="pushOn"]')).toBeChecked();
  await form.locator('input[name="pushOn"]').uncheck();
  await page.fill("#quiet-from", "22:00");
  await page.fill("#quiet-to", "07:00");
  await form.locator('button[type="submit"]').click();
  await expect(page.locator("#notify-msg")).toHaveText("Saved.");
  // Back in the chat and to Settings again: what was saved is what shows.
  await page.keyboard.press("Escape");
  await page.locator('[data-action="settings"]').first().click();
  await expect(page.locator('#notify-form input[name="pushOn"]')).not.toBeChecked();
  await expect(page.locator("#quiet-from")).toHaveValue("22:00");
  await expect(page.locator("#quiet-to")).toHaveValue("07:00");
});

test("half a quiet span is refused with a clear message", async ({ page }) => {
  await settings(page);
  await page.fill("#quiet-from", "22:00");
  await page.fill("#quiet-to", "");
  await page.locator('#notify-form button[type="submit"]').click();
  await expect(page.locator("#notify-msg")).toContainText("need a start and an end");
});

test("mail set by the server shows where it goes and hides the fields; push in this browser switches", async ({ page }) => {
  await settings(page);
  await expect(page.locator(".env-note")).toContainText("mailserver:587 (starttls)");
  await expect(page.locator(".env-note")).toContainText("KOMPANION_SMTP_");
  await expect(page.locator("#smtpHost")).toBeHidden();
  await expect(page.locator(".env-note")).not.toContainText("password");
  const toggle = page.locator("#web-push-toggle");
  await expect(toggle).toHaveText("Push in this browser");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".web-push-row [role='status']")).toContainText("also when Kompanion is closed");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-pressed", "false");
});

for (const scheme of ["light", "dark"] as const) {
  for (const width of [1280, 390]) {
    test(`notification settings fit and read well at ${width} px (${scheme})`, async ({ page }) => {
      await settings(page, scheme, width);
      await page.addStyleTag({ content: "body.demo::before{display:none!important}" });
      const form = page.locator("#notify-form");
      await form.scrollIntoViewIfNeeded();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1)).toBe(true);
      const ratios = () => page.locator("#notify-form legend, #notify-form label, #notify-form small, .web-push-row [role='status'], .env-note").evaluateAll((els) => els.filter((e) => (e as HTMLElement).offsetParent).map((el) => {
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
      if (width < 768) expect((await page.locator("#quiet-from").boundingBox())!.height).toBeGreaterThanOrEqual(44);
      await expect(async () => { await form.screenshot({ path: `test-results/notify-${width}-${scheme}.png`, timeout: 2000 }); }).toPass({ timeout: 10_000 });
    });
  }
}
