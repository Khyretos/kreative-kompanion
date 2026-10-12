import { expect, test } from "@playwright/test";

// SIDEBAR-FIX: at every sidebar width (from the 260 px minimum up) and on a 390 px phone, the values on the
// right of each row (badges, counts, times, the options button) stay fully inside the sidebar; titles give way
// with an ellipsis first.
const WIDTHS = [260, 300, 420];

async function offenders(page: import("@playwright/test").Page): Promise<string[]> {
  return page.evaluate(() => {
    const root = document.querySelector("#left") as HTMLElement;
    const nav = root.querySelector(".nav") as HTMLElement;
    const limit = nav.getBoundingClientRect().right + 0.5;
    const out: string[] = [];
    if (nav.scrollWidth > nav.clientWidth + 1) out.push(`nav scrolls sideways (${nav.scrollWidth} > ${nav.clientWidth})`);
    for (const el of root.querySelectorAll<HTMLElement>(".nav-meta, .badge, .chat-more, .project-head, .nav-item")) {
      const r = el.getBoundingClientRect();
      if (r.width === 0 || getComputedStyle(el).visibility === "hidden") continue;
      if (r.right > limit) out.push(`${el.className} "${(el.textContent ?? "").trim().slice(0, 20)}" ends at ${Math.round(r.right)} > ${Math.round(limit)}`);
    }
    // a row whose content is wider than the row itself is clipped
    for (const el of root.querySelectorAll<HTMLElement>(".project-head, .nav-item")) {
      if (el.scrollWidth > el.clientWidth + 1) out.push(`${el.className} "${(el.textContent ?? "").trim().slice(0, 20)}" clips its content (${el.scrollWidth} > ${el.clientWidth})`);
    }
    return out;
  });
}

async function open(page: import("@playwright/test").Page, left: number | null): Promise<void> {
  await page.addInitScript((w) => {
    if (w) localStorage.setItem("kk.layout", JSON.stringify({ left: w }));
  }, left);
  await page.goto("/?demo");
  await page.click("button.found-server");
  for (const head of await page.locator("#left .project-head").all()) await head.click();
}

for (const scheme of ["light", "dark"] as const) {
  for (const width of WIDTHS) {
    test(`values stay visible at sidebar width ${width} (${scheme})`, async ({ page }) => {
      await page.setViewportSize({ width: 1400, height: 900 });
      await page.emulateMedia({ colorScheme: scheme });
      await open(page, width);
      const box = await page.locator("#left").boundingBox();
      expect(Math.round(box!.width)).toBeGreaterThanOrEqual(width - 1);
      expect(await offenders(page)).toEqual([]);
    });
  }
  test(`values stay visible on a 390 px phone (${scheme})`, async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.emulateMedia({ colorScheme: scheme });
    await open(page, null);
    await page.locator('[data-action="pane"][data-pane="left"]').first().click().catch(() => undefined);
    expect(await offenders(page)).toEqual([]);
  });
}

// The right panel (tasks, machines…) has values on the right of its rows too (counts, GB, watts, states).
async function rightOffenders(page: import("@playwright/test").Page): Promise<string[]> {
  return page.evaluate(() => {
    const pane = document.querySelector("#right") as HTMLElement;
    const limit = pane.getBoundingClientRect().right - 0.5;
    const out: string[] = [];
    if (pane.scrollWidth > pane.clientWidth + 1) out.push(`pane scrolls sideways (${pane.scrollWidth} > ${pane.clientWidth})`);
    for (const el of pane.querySelectorAll<HTMLElement>("*")) {
      const cs = getComputedStyle(el);
      if (cs.display === "none" || cs.visibility === "hidden") continue;
      const own = [...el.childNodes].some((n) => n.nodeType === 3 && (n.textContent ?? "").trim() !== "");
      if (!own || cs.textOverflow === "ellipsis") continue;
      const r = el.getBoundingClientRect();
      if (r.width === 0) continue;
      if (r.right > limit) out.push(`${el.tagName.toLowerCase()}.${el.className} "${(el.textContent ?? "").trim().slice(0, 24)}" ends at ${Math.round(r.right)} > ${Math.round(limit)}`);
    }
    return out;
  });
}

for (const scheme of ["light", "dark"] as const) {
  for (const tab of ["tasks", "machines"]) {
    test(`right panel ${tab} keeps its values visible at the minimum width (${scheme})`, async ({ page }) => {
      await page.setViewportSize({ width: 1400, height: 900 });
      await page.emulateMedia({ colorScheme: scheme });
      await page.addInitScript(() => localStorage.setItem("kk.layout", JSON.stringify({ right: 260 })));
      await page.goto("/?demo");
      await page.click("button.found-server");
      await page.locator(`[data-action="tab"][data-tab="${tab}"]`).first().click();
      const box = await page.locator("#right").boundingBox();
      expect(Math.round(box!.width)).toBeLessThanOrEqual(262);
      await expect.poll(() => rightOffenders(page), { timeout: 4000 }).toEqual([]);
    });
  }
}
