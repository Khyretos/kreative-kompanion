// PROJ-ARCHIVE: a finished project is archived from its ⋯ menu in the sidebar. It leaves the list at
// once (no reload), a toast offers Undo, "Show archived (N)" brings archived projects back dimmed
// with an Archived chip and an Unarchive action. Phone width and both themes.
import { test, expect, type Page } from "@playwright/test";

async function openDemo(page: Page, width = 1280, height = 900): Promise<void> {
  await page.setViewportSize({ width, height });
  await page.addInitScript(() => {
    try { localStorage.removeItem("kk-show-archived"); } catch { /* private window */ }
  });
  await page.goto("/?demo");
  await page.click("button.found-server");
  await expect(page.locator("#left .project-head").first()).toBeAttached();
}

const head = (page: Page, id: string) => page.locator(`#left [data-action="project"][data-id="${id}"]`);
const more = (page: Page, id: string) => page.locator(`#left .project-more[data-action="project-menu"][data-id="${id}"]`);

async function archive(page: Page, id: string): Promise<void> {
  await more(page, id).click();
  await page.locator(`#left .menu [data-action="project-archive"][data-id="${id}"]`).click();
}

test("archive from the menu removes the project at once and Undo brings it back", async ({ page }) => {
  await openDemo(page);
  let loads = 0;
  page.on("load", () => { loads++; });
  await expect(more(page, "p-3dco")).toHaveAttribute("aria-label", "Options for 3dco-plus");
  await archive(page, "p-3dco");
  await expect(head(page, "p-3dco")).toHaveCount(0);
  const toast = page.locator(".toast.project-undo");
  await expect(toast).toContainText("Archived 3dco-plus");
  await toast.getByRole("button", { name: "Undo" }).click();
  await expect(head(page, "p-3dco")).toHaveCount(1);
  await expect(toast).toHaveCount(0);
  await expect(page.locator('#left [data-action="archived-toggle"]')).toHaveCount(0);
  expect(loads).toBe(0);
});

test('"Show archived (N)" shows and hides archived projects, and Unarchive restores one', async ({ page }) => {
  await openDemo(page);
  await expect(page.locator('#left [data-action="archived-toggle"]')).toHaveCount(0);
  await archive(page, "p-3dco");
  await archive(page, "p-nohboard");
  const toggle = page.locator('#left [data-action="archived-toggle"]');
  await expect(toggle).toHaveText("Show archived (2)");
  await expect(toggle).toHaveAttribute("aria-pressed", "false");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-pressed", "true");
  // Below the active ones, with a chip.
  const rows = page.locator("#left .projects > li.project");
  await expect(rows).toHaveCount(4);
  await expect(rows.nth(2)).toHaveClass(/archived/);
  await expect(rows.nth(3)).toHaveClass(/archived/);
  await expect(rows.nth(0)).not.toHaveClass(/archived/);
  await expect(page.locator("#left .project.archived .archived-chip").first()).toHaveText("Archived");
  // The toggle is remembered on this device.
  expect(await page.evaluate(() => localStorage.getItem("kk-show-archived"))).toBe("1");
  // Unarchive from the menu of an archived project.
  await more(page, "p-3dco").click();
  await page.locator('#left .menu [data-action="project-unarchive"][data-id="p-3dco"]').click();
  await expect(page.locator('#left .project.archived [data-action="project"][data-id="p-3dco"]')).toHaveCount(0);
  await expect(head(page, "p-3dco")).toHaveCount(1);
  await expect(toggle).toHaveText("Show archived (1)");
  await toggle.click();
  await expect(head(page, "p-nohboard")).toHaveCount(0);
});

test("archived projects leave the all-tasks list and the move-to-project picker", async ({ page }) => {
  await openDemo(page);
  await page.locator('[data-action="tab"][data-tab="tasks"]').first().click();
  await page.locator('#right [data-action="scope"][data-scope="all"]').click();
  await expect(page.locator("#right")).toContainText("Write unit tests for ModuleLoader");
  await archive(page, "p-kk");
  await expect(page.locator("#right")).not.toContainText("Write unit tests for ModuleLoader");
  // A loose chat's "Move to project…" list leaves the archived project out.
  const row = page.locator("#left .loose .chat-row").first();
  await row.locator('[data-action="chat-menu"]').click();
  await row.locator('[data-action="chat-move-open"]').click();
  await expect(row.locator('.submenu [data-action="chat-move"][data-project="p-kk"]')).toHaveCount(0);
  await expect(row.locator('.submenu [data-action="chat-move"][data-project="p-3dco"]')).toHaveCount(1);
});

test("an archived project that is open shows an Archived bar with Unarchive", async ({ page }) => {
  await openDemo(page);
  await head(page, "p-kk").click();
  await expect(page.locator("#right section.overview")).toBeVisible();
  await archive(page, "p-kk");
  const bar = page.locator("#right .archived-bar");
  await expect(bar).toContainText("Archived");
  await bar.locator('[data-action="project-unarchive"]').click();
  await expect(bar).toHaveCount(0);
  await expect(head(page, "p-kk")).toHaveCount(1);
});

/** WCAG contrast of an element's text against the first opaque background behind it. */
async function contrastOf(page: Page, selector: string): Promise<number> {
  return page.locator(selector).first().evaluate((el) => {
    const rgb = (c: string) => (c.match(/[\d.]+/g) ?? []).map(Number);
    const lum = ([r, g, b]: number[]) => {
      const f = (v: number) => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; };
      return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
    };
    let bg: number[] = [];
    for (let n: Element | null = el; n; n = n.parentElement) {
      const c = rgb(getComputedStyle(n).backgroundColor);
      if (c.length === 3 || (c.length === 4 && c[3] === 1)) { bg = c.slice(0, 3); break; }
    }
    if (!bg.length) bg = [255, 255, 255];
    const fg = rgb(getComputedStyle(el).color).slice(0, 3);
    const [a, b] = [lum(fg), lum(bg)].sort((x, y) => y - x);
    return (a + 0.05) / (b + 0.05);
  });
}

for (const scheme of ["light", "dark"] as const) {
  test(`phone width: the menu is reachable by touch and nothing scrolls sideways (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await openDemo(page, 390, 844);
    await page.locator('[data-action="pane"][data-pane="left"]').first().click().catch(() => undefined);
    const btn = more(page, "p-nohboard");
    await expect(btn).toBeVisible();
    const box = await btn.boundingBox();
    expect(box!.height).toBeGreaterThanOrEqual(44);
    await archive(page, "p-nohboard");
    await page.locator('#left [data-action="archived-toggle"]').click();
    await expect(page.locator("#left .project.archived")).toHaveCount(1);
    expect(await contrastOf(page, "#left .project.archived .project-name")).toBeGreaterThanOrEqual(4.5);
    const sideways = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1);
    expect(sideways).toBe(false);
    await page.locator("#left").screenshot({ path: `test-results/projarchive-390-${scheme}.png` });
  });

  test(`desktop: the archived group stays readable (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await openDemo(page);
    await archive(page, "p-3dco");
    await page.locator('#left [data-action="archived-toggle"]').click();
    expect(await contrastOf(page, "#left .project.archived .project-name")).toBeGreaterThanOrEqual(4.5);
    expect(await contrastOf(page, '#left [data-action="archived-toggle"]')).toBeGreaterThanOrEqual(4.5);
    await page.locator("#left").screenshot({ path: `test-results/projarchive-desktop-${scheme}.png` });
  });
}

// Live check 2026-10-13: the fade-in replayed on every sidebar re-render (the sidebar is remounted), so archived
// rows flickered faint whenever anything changed; and at 390 px the chip and the done/total count left the name
// only a few letters.
test("archived rows do not fade in again on a re-render, and keep room for their name", async ({ page }) => {
  await openDemo(page, 390, 844);
  await page.locator('[data-action="pane"][data-pane="left"]').first().click().catch(() => undefined);
  await archive(page, "p-nohboard");
  await page.locator('#left [data-action="archived-toggle"]').click();
  const row = page.locator("#left .project.archived").first();
  await expect(row).toBeVisible();
  for (const el of [row, page.locator("#left .project.archived .project-row").first()]) {
    expect(await el.evaluate((e) => getComputedStyle(e).animationName)).toBe("none");
  }
  await expect(row.locator(".nav-meta")).toBeHidden();
  const name = row.locator(".project-name");
  expect(await name.evaluate((e) => e.scrollWidth <= e.clientWidth + 1)).toBe(true);
});
