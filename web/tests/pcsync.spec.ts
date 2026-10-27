import { test, expect } from "@playwright/test";

// PC-SYNC (Kees, 2026-10-27): the computer picked for a chat belongs to the chat, not to one
// browser, so a phone opening the same chat shows the same computer.
test("the computer picked for a chat comes back from the chat, not from this browser", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="open-chat"][data-id="c-kk"]');
  await page.selectOption("#pc-machine", { label: "soucouyant" });
  await page.click('[data-action="open-chat"][data-id="c-kk-2"]');
  // Another device: nothing remembered in its browser.
  await page.evaluate(() => { localStorage.removeItem("kk-pc-chats"); localStorage.removeItem("kk-pc-last"); });
  await page.click('[data-action="open-chat"][data-id="c-kk"]');
  await expect(page.locator("#pc-machine option:checked")).toHaveText("soucouyant");
  await page.selectOption("#pc-machine", { index: 0 });
  await page.click('[data-action="open-chat"][data-id="c-kk-2"]');
  await page.click('[data-action="open-chat"][data-id="c-kk"]');
  await expect(page.locator("#pc-machine")).toHaveValue("");
});
