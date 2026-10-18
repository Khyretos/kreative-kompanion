import { test, expect, type Page } from "@playwright/test";

// MACHINES-CARDS (Kees, 2026-10-18): the Access tab must show which checkboxes and buttons belong to which
// computer. Each computer is a card with its name as a real header; its folder and system forms sit inside it,
// are named after it, and keep their own typed text when the page refreshes live.
async function openAccess(page: Page): Promise<void> {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.locator('[data-action="tab"][data-tab="access"]').first().click();
  await expect(page.locator(".machine-card").first()).toBeVisible();
}
const card = (page: Page, id: string) => page.locator(`section.machine-card[data-machine="${id}"]`);

test("each computer is a card headed by its name, with its own folder and system forms", async ({ page }) => {
  await openAccess(page);
  for (const [id, name] of [["kireserver", "kireserver"], ["soucouyant", "soucouyant"], ["desktop-387pphp", "DESKTOP-387PPHP"]]) {
    const c = card(page, id);
    await expect(c.getByRole("heading", { level: 3, name })).toBeVisible();
    await expect(c).toHaveAttribute("aria-labelledby", `access-${id}-name`);
    await expect(c.locator(`#access-${id}-name`)).toHaveText(name);
    const folder = c.locator(`form.grant-add[data-id="${id}|folder"]`);
    const system = c.locator(`form.grant-add[data-id="${id}|system"]`);
    await expect(folder).toHaveAttribute("aria-label", `Folder access on ${name}`);
    await expect(system).toHaveAttribute("aria-label", `System access on ${name}`);
    await expect(folder.locator(".grant-block-title")).toHaveText("Folder access");
    await expect(system.locator(".grant-block-title")).toHaveText("System access");
    await expect(folder.locator('button[type="submit"]')).toHaveText("Grant folder access");
    await expect(system.locator('button[type="submit"]')).toHaveText("Grant system access");
    await expect(folder.locator('input[name="target"]')).toHaveAttribute("aria-label", `Folder on ${name}`);
  }
});

test("a Windows computer gets a Windows folder example, the others a Linux one", async ({ page }) => {
  await openAccess(page);
  await expect(card(page, "desktop-387pphp").locator('form[data-id="desktop-387pphp|folder"] input[name="target"]')).toHaveAttribute("placeholder", "C:\\Users\\you\\projects");
  await expect(card(page, "kireserver").locator('form[data-id="kireserver|folder"] input[name="target"]')).toHaveAttribute("placeholder", "/home/you/projects/app");
});

test("text typed for one computer stays on that computer after a live refresh", async ({ page }) => {
  await openAccess(page);
  const win = card(page, "desktop-387pphp").locator('form[data-id="desktop-387pphp|folder"]');
  await win.locator('input[name="target"]').fill("C:\\kk-runner");
  await win.locator('select[name="expires"]').selectOption("");
  // The machines list refreshes every few seconds and re-renders this tab.
  await page.waitForTimeout(7000);
  await expect(win.locator('input[name="target"]')).toHaveValue("C:\\kk-runner");
  await expect(win.locator('select[name="expires"]')).toHaveValue("");
  for (const id of ["kireserver", "soucouyant"]) {
    await expect(card(page, id).locator(`form[data-id="${id}|folder"] input[name="target"]`)).toHaveValue("");
    await expect(card(page, id).locator(`form[data-id="${id}|folder"] select[name="expires"]`)).toHaveValue("24");
    await expect(card(page, id).locator(`form[data-id="${id}|system"] input[name="target"]`)).toHaveValue("system");
  }
  await expect(card(page, "desktop-387pphp").locator('form[data-id="desktop-387pphp|system"] input[name="target"]')).toHaveValue("system");
});

for (const scheme of ["dark", "light"] as const) {
  test(`card headers are bright and high-contrast (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await openAccess(page);
    const head = card(page, "soucouyant").locator(".machine-card-head");
    const ratio = await head.evaluate((el) => {
      const rgb = (s: string) => (s.match(/[\d.]+/g) ?? []).slice(0, 3).map(Number);
      const lum = (c: number[]) => { const [r, g, b] = c.map((v) => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; }); return 0.2126 * r + 0.7152 * g + 0.0722 * b; };
      const fg = lum(rgb(getComputedStyle(el.querySelector("h3")!).color));
      // The first ancestor with a painted background is what the header text sits on.
      let box: Element | null = el;
      let paint = "rgb(0, 0, 0)";
      while (box) { const c = getComputedStyle(box).backgroundColor; if (!/rgba\(.*, 0\)$/.test(c) && c !== "transparent") { paint = c; break; } box = box.parentElement; }
      const bg = lum(rgb(paint));
      return (Math.max(fg, bg) + 0.05) / (Math.min(fg, bg) + 0.05);
    });
    expect(ratio).toBeGreaterThanOrEqual(7);
    // The name sits on its own coloured band, so where one computer starts is visible at a glance.
    const [headBg, bodyBg] = await card(page, "soucouyant").evaluate((el) => [getComputedStyle(el.querySelector(".machine-card-head")!).backgroundColor, getComputedStyle(el.querySelector(".machine-card-body")!).backgroundColor]);
    expect(headBg).not.toMatch(/rgba\(.*, 0\)$/);
    expect(headBg).not.toBe(bodyBg);
    const border = await card(page, "soucouyant").evaluate((el) => getComputedStyle(el).borderTopWidth);
    expect(parseFloat(border)).toBeGreaterThanOrEqual(2);
    const text = await card(page, "soucouyant").locator(".grant-rights label").first().evaluate((el) => getComputedStyle(el).color);
    expect(text).not.toBe("rgba(0, 0, 0, 0)");
  });
}

test("cards fit a 390 px phone without sideways scrolling", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openAccess(page);
  const over = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(over).toBeLessThanOrEqual(0);
  const box = await card(page, "desktop-387pphp").boundingBox();
  expect(box!.x).toBeGreaterThanOrEqual(0);
  expect(box!.x + box!.width).toBeLessThanOrEqual(390);
});

test("no card cuts off its own forms, also when the list is longer than the pane", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openAccess(page);
  // One evaluate per card: the demo re-renders the pane every few seconds, which detaches held elements.
  const n = await page.locator(".machine-card").count();
  for (let i = 0; i < n; i++) {
    const [cut, seen] = await page.locator(".machine-card").nth(i).evaluate((el) => {
      const b = [...el.querySelectorAll('button[type="submit"]')].pop() as HTMLElement;
      b.scrollIntoView({ block: "center" });
      const r = b.getBoundingClientRect();
      return [el.scrollHeight - el.clientHeight, r.top >= 0 && r.bottom <= innerHeight && r.height > 0];
    });
    expect(cut).toBeLessThanOrEqual(1);
    expect(seen).toBe(true);
  }
});
