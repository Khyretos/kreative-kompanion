import { test, expect } from "@playwright/test";

// RUNNER-SYNC: a paired computer whose runner is older than the server's shows an Update button
// (not just a chip); clicking it queues the update and the button says so. Phone width, both themes.
for (const scheme of ["light", "dark"] as const) {
  test(`an outdated runner has an Update button (${scheme}, 390 px)`, async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto("/?demo");
    await page.click("button.found-server");
    await page.locator('[data-action="tab"][data-tab="machines"]').first().click();
    const card = page.locator("ul.machines > li.machine").filter({ hasText: "soucouyant" });
    const button = card.locator('[data-action="update-runner"]');
    await expect(button).toHaveText("Update to 0.6.0");
    const box = await button.boundingBox();
    expect(box && box.x >= 0 && box.x + box.width <= 390).toBe(true);
    // up to date or the server's own card: no button
    await expect(page.locator("ul.machines > li.machine").filter({ hasText: "kireserver" }).locator('[data-action="update-runner"]')).toHaveCount(0);
    await button.click();
    await expect(button).toHaveText("Updating…");
    await expect(button).toBeDisabled();
  });
}
