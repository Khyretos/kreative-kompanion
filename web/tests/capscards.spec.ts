import { test, expect, type Page } from "@playwright/test";

// CAP-02 (Kees, 2026-10-07): capabilities cards stylish, no text overflow, equal height per row.
async function openCaps(page: Page): Promise<void> {
  await page.goto("/?demo");
  await page.click("button.found-server");
  if ((page.viewportSize()?.width ?? 1280) < 760) await page.click('#conv-head [data-action="pane"][data-pane="left"]');
  await page.click('[data-action="capabilities"]');
  await expect(page.locator("#caps .cap").first()).toBeVisible();
}

/** Cards (or anything inside them) whose content is wider than the box. */
function overflowing(page: Page): Promise<string[]> {
  return page.evaluate(() => [...document.querySelectorAll<HTMLElement>("#caps .cap, #caps .cap *")]
    .filter((el) => el.offsetParent !== null && !el.classList.contains("sr-only") && getComputedStyle(el).overflowX !== "auto" && el.scrollWidth > el.clientWidth + 1 && el.clientWidth > 0)
    .map((el) => `${el.className || el.tagName}: ${(el.textContent ?? "").trim().slice(0, 50)} (${el.scrollWidth} > ${el.clientWidth})`));
}

for (const width of [375, 1280]) {
  test(`no capabilities card overflows at ${width} px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await openCaps(page);
    expect(await overflowing(page)).toEqual([]);
    const pageOverflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(pageOverflow).toBeLessThanOrEqual(0);
  });
}

test("capabilities cards have a header with an icon, title and state badge, and equal heights per row", async ({ page }) => {
  await openCaps(page);
  const cards = page.locator("#caps .caps-cards > .cap");
  const n = await cards.count();
  expect(n).toBeGreaterThan(5);
  for (let i = 0; i < n; i++) {
    const head = cards.nth(i).locator(".cap-head");
    await expect(head.locator("svg").first()).toBeVisible();
    await expect(head.locator(".cap-title")).not.toBeEmpty();
    await expect(head.locator(".chip.state")).toHaveCount(1);
  }
  // Cards sit on a raised surface, not on the page colour.
  const [cardBg, pageBg] = await page.evaluate(() => [getComputedStyle(document.querySelector("#caps .cap")!).backgroundColor, getComputedStyle(document.querySelector("#caps")!).backgroundColor]);
  expect(cardBg).not.toBe(pageBg);
  const rows = await page.evaluate(() => {
    const out: Record<string, number[]> = {};
    for (const ul of document.querySelectorAll("#caps .caps-cards")) for (const li of ul.children) {
      const r = li.getBoundingClientRect();
      (out[`${Math.round(r.top)}`] ??= []).push(Math.round(r.height));
    }
    return Object.values(out);
  });
  for (const hs of rows) expect(new Set(hs).size).toBe(1);
});

test("long card text is clamped with a details toggle", async ({ page }) => {
  await openCaps(page);
  const more = page.locator("#caps .cap details.cap-more").first();
  await expect(more).toBeVisible();
  await expect(more).not.toHaveAttribute("open", "");
  await more.locator("summary").click();
  await expect(more).toHaveAttribute("open", "");
});
