import { test, expect, type Page } from "@playwright/test";

// PROV-UI: admins add, change and remove model providers in Settings; New project in the sidebar.
async function settings(page: Page, scheme: "light" | "dark" = "dark", width = 1280): Promise<void> {
  await page.emulateMedia({ colorScheme: scheme });
  await page.setViewportSize({ width, height: 900 });
  await page.goto("/?demo");
  await page.click("button.found-server");
  // On a phone the sidebar (with Settings) slides in from the ☰ first.
  if (width < 768) await page.locator('[data-action="pane"][data-pane="left"]').first().click().catch(() => undefined);
  await page.locator('[data-action="settings"]').first().click();
  await expect(page.locator(".providers-settings")).toBeVisible();
  await expect(page.locator('.provider[data-provider="ovms"]')).toContainText("Set in kompanion.toml");
}

test("the xAI Grok preset adds a provider whose models every list offers at once", async ({ page }) => {
  page.on("dialog", (d) => d.accept());
  await settings(page);
  await page.click('[data-action="add-provider"]');
  await page.selectOption("#prov-preset", "xai");
  await expect(page.locator("#prov-name")).toHaveValue("xAI Grok");
  await expect(page.locator("#prov-url")).toHaveValue("https://api.x.ai/v1");
  await expect(page.locator("#prov-env")).toHaveValue("XAI_API_KEY");
  await expect(page.locator("#provider-form")).toContainText("never shows or stores the key");
  await page.locator('#provider-form button[type="submit"]').click();
  const row = page.locator('.provider[data-provider="xai-grok"]');
  await expect(row).toContainText("key from XAI_API_KEY");
  await expect(row.locator(".model-row code").first()).toHaveText("grok-4");
  // The role pickers follow without a reload.
  await expect(page.locator('#role-worker option', { hasText: "grok-4 · xAI Grok" })).toHaveCount(1);
  await expect(page.locator(".provider-form")).toHaveCount(0);
  await expect(row.locator('[data-action="provider-edit"]')).toBeFocused();
  // Change it, then remove it.
  await row.locator('[data-action="provider-edit"]').click();
  await page.fill("#prov-name", "Grok");
  await page.locator('#provider-form button[type="submit"]').click();
  await expect(row.locator(".provider-head strong")).toHaveText("Grok");
  await row.locator('[data-action="provider-remove"]').click();
  await expect(row).toHaveCount(0);
  // The config file's providers have no Change or Remove.
  await expect(page.locator('.provider[data-provider="ovms"] [data-action="provider-edit"]')).toHaveCount(0);
  await expect(page.locator('.provider[data-provider="ovms"]')).toContainText("Set in kompanion.toml");
});

test("a key name that is not meant for keys is refused in the form; a secret file shows where it may live", async ({ page }) => {
  await settings(page);
  await page.click('[data-action="add-provider"]');
  await page.selectOption("#prov-preset", "other");
  await page.fill("#prov-name", "My server");
  await page.fill("#prov-url", "https://llm.example.lan/v1");
  await page.fill("#prov-env", "DATABASE_URL");
  await page.locator('#provider-form button[type="submit"]').click();
  await expect(page.locator('#provider-form [role="alert"]')).toContainText("ends in _KEY or _TOKEN");
  // What was typed stays.
  await expect(page.locator("#prov-name")).toHaveValue("My server");
  await page.check('#provider-form input[name="keySource"][value="file"]');
  await expect(page.locator("#prov-file")).toBeVisible();
  await expect(page.locator("#provider-form")).toContainText("/run/secrets");
  await expect(page.locator("#prov-name")).toHaveValue("My server");
  await page.fill("#prov-file", "/run/secrets/my_key");
  await page.locator('#provider-form button[type="submit"]').click();
  await expect(page.locator('.provider[data-provider="my-server"]')).toContainText("key from /run/secrets/my_key");
  // Cancel closes the form and gives focus back.
  await page.click('[data-action="add-provider"]');
  await page.click('[data-action="provider-cancel"]');
  await expect(page.locator("#provider-form")).toHaveCount(0);
  await expect(page.locator('[data-action="add-provider"]')).toBeFocused();
});

test("New project in the sidebar makes a project and opens it; the same name twice is refused", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="new-project"]');
  await expect(page.locator("#new-project-name")).toBeFocused();
  await page.fill("#new-project-name", "Medabots");
  await page.selectOption("#new-project-type", "game");
  await page.press("#new-project-name", "Enter");
  const head = page.locator('#left [data-action="project"]', { hasText: "Medabots" });
  await expect(head).toHaveClass(/active/);
  await expect(head).toBeFocused();
  await expect(page.locator("#new-project-form")).toHaveCount(0);
  await page.click('[data-action="new-project"]');
  await page.fill("#new-project-name", "medabots");
  await page.press("#new-project-name", "Enter");
  await expect(page.locator('#new-project-form [role="alert"]')).toContainText("already have a project");
  await page.click('[data-action="new-project-cancel"]');
  await expect(page.locator('[data-action="new-project"]')).toBeFocused();
});

for (const scheme of ["light", "dark"] as const) {
  for (const width of [1280, 390]) {
    test(`the provider form and New project fit and read well at ${width} px (${scheme})`, async ({ page }) => {
      await settings(page, scheme, width);
      await page.addStyleTag({ content: "body.demo::before{display:none!important}" });
      await page.click('[data-action="add-provider"]');
      await page.selectOption("#prov-preset", "xai");
      // The preset redraws the form on the next frame: measure that one.
      await expect(page.locator("#prov-name")).toHaveValue("xAI Grok");
      const form = page.locator("#provider-form");
      await expect(form).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1)).toBe(true);
      // Polled like the contrast below: a live update can redraw the form between two reads.
      await expect.poll(async () => { const b = await form.boundingBox(); return b ? b.x >= 0 && b.x + b.width <= width : null; }).toBe(true);
      // Polled: a live update can redraw the form while it is measured (a detached node reads NaN).
      const ratios = () => form.locator("label, small, legend, h4, p").evaluateAll((els) => els.map((el) => {
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
        return { text: el.textContent?.trim().slice(0, 40), ratio: (a + 0.05) / (b + 0.05) };
      }));
      await expect.poll(async () => Math.min(...(await ratios()).map((r) => r.ratio))).toBeGreaterThanOrEqual(4.5);
      await expect(async () => { await form.screenshot({ path: `test-results/provider-form-${width}-${scheme}.png`, timeout: 2000 }); }).toPass({ timeout: 10_000 });
    });
  }
}
