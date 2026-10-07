import { test, expect } from "@playwright/test";

// HOST-01: the server's own computer is one card (its paired runner takes over the server entry), and
// Pair offers "This server's computer" until it is paired.
test("the server's computer shows once, marked as this server", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.locator('[data-action="tab"][data-tab="machines"]').first().click();
  const cards = page.locator("ul.machines > li.machine");
  await expect(cards.filter({ hasText: "kireserver" })).toHaveCount(1);
  await expect(cards.filter({ hasText: "kireserver" }).locator(".chip.this-server")).toHaveText("this server");
  await expect(page.locator('[data-action="pair-host"]')).toHaveCount(0);
});

test("without it, Pair offers the server's computer by name", async ({ page }) => {
  await page.addInitScript(() => { (globalThis as { __kkDemoNoHost?: boolean }).__kkDemoNoHost = true; });
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.locator('[data-action="tab"][data-tab="machines"]').first().click();
  const host = page.locator('[data-action="pair-host"]');
  await expect(host).toHaveText("This server's computer (kireserver)");
  await host.click();
  await expect(page.locator(".pair-result")).toContainText("Run this on kireserver");
});
