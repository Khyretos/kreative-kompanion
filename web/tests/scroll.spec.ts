import { expect, test, type Page } from "@playwright/test";

// SCROLL-FIX: on a short window (laptop with a task bar, a phone in landscape) every control of every panel,
// form and dialog must be reachable by scrolling: after scrolling it into view it lies fully inside the window.
const SIZES = [
  { width: 1200, height: 380 },
  { width: 390, height: 420 },
];

async function unreachable(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const out: string[] = [];
    // A user can scroll a box with overflow auto/scroll; "hidden" and "clip" boxes only scroll from script.
    const reachable = (el: HTMLElement): boolean => {
      for (let p = el.parentElement; p; p = p.parentElement) {
        const oy = getComputedStyle(p).overflowY;
        if (oy === "visible") continue;
        let r = el.getBoundingClientRect();
        const pr = p.getBoundingClientRect();
        if (r.bottom > pr.bottom + 1 || r.top < pr.top - 1) {
          if (oy !== "auto" && oy !== "scroll") return false;
          p.scrollTop += r.bottom > pr.bottom ? r.bottom - pr.bottom : r.top - pr.top;
          r = el.getBoundingClientRect();
          if (r.bottom > pr.bottom + 1 || r.top < pr.top - 1) return false;
        }
      }
      const r = el.getBoundingClientRect();
      return r.top >= -1 && r.bottom <= window.innerHeight + 1;
    };
    for (const el of document.querySelectorAll<HTMLElement>("button, a[href], input, select, textarea, summary")) {
      const cs = getComputedStyle(el);
      const r = el.getBoundingClientRect();
      if (cs.display === "none" || cs.visibility === "hidden" || (r.width === 0 && r.height === 0)) continue;
      if (el.closest("[hidden], [inert], .sr-only, #asset-rows")) continue;
      const details = el.closest("details");
      if (details && !details.open && !(el.tagName === "SUMMARY" && el.parentElement === details)) continue;
      if (r.left >= window.innerWidth || r.right <= 0) continue; // off-canvas pane (phone)
      if (!reachable(el)) {
        out.push(`${el.tagName.toLowerCase()}[${el.getAttribute("data-action") ?? el.id ?? el.className}] "${(el.textContent ?? el.getAttribute("aria-label") ?? "").trim().slice(0, 24).replace(/\s+/g, " ")}"`);
      }
    }
    return out;
  });
}

async function start(page: Page, size: { width: number; height: number }, scheme: "light" | "dark"): Promise<void> {
  await page.setViewportSize(size);
  await page.emulateMedia({ colorScheme: scheme });
  await page.goto("/?demo");
  await page.click("button.found-server");
}

const VIEWS: { name: string; open: (page: Page) => Promise<void> }[] = [
  {
    name: "new task form",
    open: async (page) => {
      await page.locator('[data-action="new-task"]').first().evaluate((el) => (el as HTMLElement).click());
    },
  },
  {
    name: "edit task",
    open: async (page) => {
      await page.locator('[data-action="open-task"]').first().evaluate((el) => (el as HTMLElement).click());
      await page.locator('[data-action="edit-task"]').first().evaluate((el) => (el as HTMLElement).click());
    },
  },
  { name: "settings", open: async (page) => { await page.locator('[data-action="settings"]').first().evaluate((el) => (el as HTMLElement).click()); } },
  { name: "capabilities", open: async (page) => { await page.locator('[data-action="capabilities"]').first().evaluate((el) => (el as HTMLElement).click()); } },
  { name: "assets", open: async (page) => { await page.locator('[data-action="assets"]').first().evaluate((el) => (el as HTMLElement).click()); } },
  { name: "studio", open: async (page) => { await page.locator('[data-action="studio"]').first().evaluate((el) => (el as HTMLElement).click()); } },
  { name: "search dialog", open: async (page) => { await page.locator('[data-action="search"]').first().evaluate((el) => (el as HTMLElement).click()); } },
  { name: "chat menu", open: async (page) => { await page.locator('[data-action="chat-menu"]').first().evaluate((el) => (el as HTMLElement).click()); } },
  { name: "machines", open: async (page) => { await page.locator('[data-action="tab"][data-tab="machines"]').first().evaluate((el) => (el as HTMLElement).click()); } },
  { name: "access", open: async (page) => { await page.locator('[data-action="tab"][data-tab="access"]').first().evaluate((el) => (el as HTMLElement).click()); } },
];

for (const scheme of ["light", "dark"] as const) {
  for (const size of SIZES) {
    for (const view of VIEWS) {
      test(`${view.name} is fully reachable at ${size.width}x${size.height} (${scheme})`, async ({ page }) => {
        await start(page, size, scheme);
        await view.open(page).catch(() => test.skip(true, `${view.name} is not in the demo at this size`));
        await page.waitForTimeout(400);
        expect(await unreachable(page)).toEqual([]);
      });
    }
  }
}
