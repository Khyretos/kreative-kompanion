import { test, expect } from "@playwright/test";

// TEN-05: the task detail shows what a task cost; Activity shows the last 7 days.
test("a task with a cost line shows Coder's share", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="project"][data-id="p-kk"]');
  await page.locator(".tasks .task-main", { hasText: "Profile shader compile times" }).click();
  await expect(page.locator(".task-costs")).toContainText("Coder's share");
  await expect(page.locator(".task-costs")).toContainText("14 %");
});

test("Activity shows the weekly totals", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="tab"][data-tab="activity"]');
  await expect(page.locator(".activity-costs")).toContainText("Last 7 days");
  // UI-01: its own framed card like the activity cards, the numbers as label/value pairs.
  const card = page.locator(".act-week");
  await expect(card.locator(".act-week-head")).toHaveText("Last 7 days");
  await expect(card.locator(".act-week-stats dt")).toHaveText(["Coder", "Claude", "Tasks", "Coder's share"]);
  await expect(card.locator(".act-week-stats dd")).toHaveText(["53,100 tokens", "301,600 tokens", "5", "15 %"]);
  const look = await card.evaluate((el) => {
    const c = getComputedStyle(el), a = getComputedStyle(document.querySelector(".act-card")!);
    return { border: c.borderTopStyle, bg: c.backgroundColor === a.backgroundColor, radius: c.borderTopLeftRadius === a.borderTopLeftRadius };
  });
  expect(look).toEqual({ border: "solid", bg: true, radius: true });
});

for (const width of [390, 1280]) {
  test(`the weekly card is not cut off at ${width} px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/?demo");
    await page.click("button.found-server");
    if (width < 760) await page.click('#conv-head [data-action="pane"][data-pane="right"]');
    await page.locator('[data-action="tab"][data-tab="activity"]').first().click();
    const card = page.locator(".act-week");
    await expect(card).toBeVisible();
    const [box, pane] = await Promise.all([card.boundingBox(), page.locator("#right").boundingBox()]);
    expect(box!.x).toBeGreaterThanOrEqual(pane!.x);
    expect(box!.x + box!.width).toBeLessThanOrEqual(pane!.x + pane!.width + 0.5);
    expect(await card.evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(true);
  });
}
