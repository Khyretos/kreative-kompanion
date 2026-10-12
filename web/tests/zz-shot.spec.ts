import { test } from "@playwright/test";
for (const scheme of ["light", "dark"] as const) {
  test(`kg10 shot ${scheme}`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await page.setViewportSize({ width: 390, height: 900 });
    await page.goto("/?demo");
    await page.click("button.found-server");
    await page.click('[data-action="project"][data-id="p-kk"]');
    await page.selectOption("#project-type", "game");
    await page.locator("#dev-preset").selectOption("game");
    await page.locator("form.project-dev").scrollIntoViewIfNeeded();
    await page.screenshot({ path: `test-results/kg10-${scheme}.png` });
  });
}
