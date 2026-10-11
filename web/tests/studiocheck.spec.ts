import { test, expect } from "@playwright/test";

// STU-C4: a checked OC sheet says so at a glance; problems that are left open on click.
for (const scheme of ["light", "dark"] as const) {
  test(`anatomy check line on sheet cards (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto("/?demo");
    await page.click("button.found-server");
    await page.click('[data-action="studio"]');
    await page.setViewportSize({ width: 390, height: 844 });
    const passed = page.locator('#studio .studio-run:has(img[alt*="silver hair"]) .studio-check');
    await expect(passed).toHaveText("Anatomy checked · re-rolled 2 times");
    const left = page.locator('#studio .studio-run:has(img[alt*="red hoodie"]) details.studio-check');
    await expect(left.locator("summary")).toHaveText("Anatomy: 1 problem left after 3 re-rolls");
    await expect(left.locator("li")).toBeHidden();
    await left.locator("summary").click();
    await expect(left.locator("li")).toHaveText("back: face seen (head facing the wrong way)");
    await expect(page.locator('#studio .studio-run:has(img[alt*="cheerful fox"]) .studio-check')).toHaveCount(0);
    await left.scrollIntoViewIfNeeded();
    await page.screenshot({ path: `test-results/studiocheck-${scheme}.png` });
  });
}
