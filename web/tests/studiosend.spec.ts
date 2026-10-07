import { test, expect } from "@playwright/test";

// STU-02b: a finished Studio result goes to one of your projects' assets, with its provenance.
test("Send to Assets puts a Studio result into a project", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="studio"]');
  const run = page.locator("#studio .studio-run.s-done").first();
  await expect(run).toBeVisible();
  const send = run.locator("details.studio-send");
  await send.locator("summary").click();
  await expect(send.locator('[data-action="studio-send"]')).toHaveText(["Kreative Kompanion", "kk-engine", "3dco-plus", "nohboard-qt"]);
  await send.locator('[data-action="studio-send"]', { hasText: "kk-engine" }).click();
  await expect(run.locator(".studio-sent")).toHaveText("In Assets: kk-engine");
  // Sending it to a second project lists both.
  await run.locator("details.studio-send summary").click();
  await run.locator('[data-action="studio-send"]', { hasText: "3dco-plus" }).click();
  await expect(run.locator(".studio-sent")).toHaveText("In Assets: kk-engine, 3dco-plus");
});

test("An asset made in the Studio shows where it came from", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="assets"]');
  // Demo asset 1 (Action_01.mp3) stands in for a song made in the Studio.
  await page.fill("#asset-q", "fantasy action 01");
  await page.locator(".asset-card:not(.skeleton)", { hasText: "Action_01.mp3" }).first().click();
  const prov = page.locator(".asset-provenance");
  await expect(prov.locator("h3")).toHaveText("Made in the Studio");
  await expect(prov).toContainText("Music");
  await expect(prov).toContainText("calm lofi piano loop");
  await expect(prov).toContainText("seed 7");
  await expect(prov).toContainText("rx9070");
  await expect(prov).toContainText("HeartMuLa (Apache-2.0)");
  // A file from the library has no such box.
  await page.fill("#asset-q", "town 02");
  await page.locator(".asset-card:not(.skeleton)", { hasText: "Town_02.mp3" }).first().click();
  await expect(page.locator(".asset-detail h2, #asset-detail h2").first()).toContainText("Town_02");
  await expect(page.locator(".asset-provenance")).toHaveCount(0);
});
