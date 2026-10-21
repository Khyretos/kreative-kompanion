import { test, expect, type Page } from "@playwright/test";

// 3D-01: a finished "3D model" run shows View in 3D and a .glb download, and opens the 3D viewer.
async function open3d(page: Page, scheme: "light" | "dark", phone = true) {
  await page.emulateMedia({ colorScheme: scheme });
  // Open the Studio at the default width, then go to phone width (the open left pane would cover the cards).
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="studio"]');
  if (phone) await page.setViewportSize({ width: 390, height: 844 });
  const card = page.locator('#studio .studio-run[data-type="image-to-3d"]');
  await expect(card).toHaveCount(1);
  return card;
}

for (const scheme of ["light", "dark"] as const) {
  test(`3D card shows View in 3D and a download (${scheme}, 390 px)`, async ({ page }) => {
    const card = await open3d(page, scheme);
    // The preview render is the card picture; the .glb gets the buttons below it.
    await expect(card.locator("img")).toHaveCount(1);
    await expect(card.locator("img")).toHaveAttribute("alt", /3d model/);
    await expect(card.locator('button[data-action="studio-3d"]')).toHaveCount(1);
    await expect(card.locator('button[data-action="studio-3d"]')).toHaveText("View in 3D");
    await expect(card.locator("a[download]")).toHaveText("Download .glb");
    await expect(card.locator("a[download]")).toHaveAttribute("download", /\.glb$/);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await card.scrollIntoViewIfNeeded();
    await page.screenshot({ path: `test-results/studio3d-card-${scheme}.png` });
  });

  // At 390 px the left pane overlays the list after the switch, so the dialog is checked at desktop width.
  test(`View in 3D opens the model viewer (${scheme})`, async ({ page }) => {
    const card = await open3d(page, scheme, false);
    await card.locator('button[data-action="studio-3d"]').click();
    const dialog = page.locator("dialog.asset-viewer.kind-model");
    await expect(dialog).toBeVisible();
    await expect(dialog.locator("h2")).toHaveText(/\.glb$/);
    await expect(dialog.locator("canvas")).toHaveCount(1, { timeout: 15000 });
    await page.screenshot({ path: `test-results/studio3d-viewer-${scheme}.png` });
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
  });
}
