import { test, expect, type Page } from "@playwright/test";

// ACC-01 (Kees, 2026-10-06): one list of what Kompanion may do per computer: times, renew, permanent
// only after an explicit "I understand", expired grants kept for a week.
async function openAccess(page: Page): Promise<void> {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.locator('[data-action="tab"][data-tab="access"]').first().click();
  await expect(page.locator(".grant-row").first()).toBeVisible();
}
const souc = (page: Page) => page.locator('section.group[data-machine="soucouyant"]');

test("grants show date and time, sorted by expiry, expired ones folded away", async ({ page }) => {
  await openAccess(page);
  const rows = souc(page).locator("ul.grants > .grant-row");
  await expect(rows.locator("code")).toHaveText(["system", "/home/kees/projects/kompanion"]);
  await expect(rows.first().locator(".grant-when")).toContainText(/expires in (2|3) h \(/);
  await expect(rows.nth(1).locator(".grant-when .chip.permanent")).toHaveText("permanent");
  await expect(rows.first().locator(".chip.root")).toHaveText("root");
  const expired = souc(page).locator("details.grants-expired");
  await expect(expired.locator("summary")).toHaveText("Expired (1)");
  await expect(expired).not.toHaveAttribute("open", "");
  await expired.locator("summary").click();
  await expect(expired.locator(".grant-row code")).toHaveText("/home/kees/old-notes");
  await expect(expired.locator('[data-action="grant-renew"][data-hours="24"]')).toHaveText("Grant again");
});

test("the summary counts active, permanent and root grants per computer", async ({ page }) => {
  await openAccess(page);
  const row = page.locator('.access-summary [data-machine="soucouyant"]');
  await expect(row).toContainText("2 active");
  await expect(row).toContainText("1 permanent");
  await expect(row).toContainText("1 with root");
});

test("renew for a day keeps the rights and moves the expiry", async ({ page }) => {
  await openAccess(page);
  const sys = souc(page).locator("ul.grants > .grant-row", { has: page.locator("code", { hasText: "system" }) });
  await sys.locator("details.grant-renew summary").click();
  await sys.locator('[data-action="grant-renew"][data-hours="24"]').click();
  await expect(sys.locator(".grant-when")).toContainText(/expires in (23|24) h/);
  await expect(sys.locator(".grant-rights .chip")).toHaveText(["packages", "root"]);
});

test("permanent needs the I-understand tick; cancel changes nothing", async ({ page }) => {
  await openAccess(page);
  const sys = souc(page).locator("ul.grants > .grant-row", { has: page.locator("code", { hasText: "system" }) });
  await sys.locator("details.grant-renew summary").click();
  await sys.locator('[data-action="grant-renew"][data-hours=""]').click();
  const dlg = page.locator("dialog.perm-dialog");
  await expect(dlg).toBeVisible();
  await expect(dlg).toContainText("You are responsible for managing it");
  const ok = dlg.locator('button[value="confirm"]');
  await expect(ok).toBeDisabled();
  await dlg.locator('button[value="cancel"]').click();
  await expect(dlg).toHaveCount(0);
  await expect(sys.locator(".grant-when")).toContainText("expires in");
  // Again, with the tick.
  await sys.locator("details.grant-renew summary").click();
  await sys.locator('[data-action="grant-renew"][data-hours=""]').click();
  await page.locator('dialog.perm-dialog input[name="understand"]').check();
  await page.locator('dialog.perm-dialog button[value="confirm"]').click();
  await expect(sys.locator(".grant-when .chip.permanent")).toHaveText("permanent");
});

test("the folder form defaults to one day and asks before a permanent grant", async ({ page }) => {
  await openAccess(page);
  const form = souc(page).locator("form.grant-add").first();
  await expect(form.locator('select[name="expires"]')).toHaveValue("24");
  await form.locator('input[name="target"]').fill("/home/kees/new");
  await form.locator('select[name="expires"]').selectOption("");
  await form.locator('button[type="submit"]').click();
  await expect(page.locator("dialog.perm-dialog")).toBeVisible();
  await page.locator('dialog.perm-dialog button[value="cancel"]').click();
  await expect(souc(page).locator(".grant-row code", { hasText: "/home/kees/new" })).toHaveCount(0);
  await form.locator('button[type="submit"]').click();
  await page.locator('dialog.perm-dialog input[name="understand"]').check();
  await page.locator('dialog.perm-dialog button[value="confirm"]').click();
  await expect(souc(page).locator(".grant-row", { hasText: "/home/kees/new" }).locator(".chip.permanent")).toHaveText("permanent");
});

test("revoke still works", async ({ page }) => {
  await openAccess(page);
  page.once("dialog", (d) => d.accept());
  await souc(page).locator('.grant-row:has(code:text-is("/home/kees/projects/kompanion")) [data-action="grant-revoke"]').click();
  await expect(souc(page).locator('ul.grants .grant-row code', { hasText: "/home/kees/projects/kompanion" })).toHaveCount(0);
});

test("an open Renew menu and the Expired list stay open while the page refreshes live", async ({ page }) => {
  await openAccess(page);
  const sys = souc(page).locator("ul.grants > .grant-row", { has: page.locator("code", { hasText: "system" }) });
  await sys.locator("details.grant-renew summary").click();
  await souc(page).locator("details.grants-expired summary").click();
  // The machines list refreshes every few seconds and re-renders this tab.
  await page.waitForTimeout(7000);
  await expect(sys.locator("details.grant-renew")).toHaveAttribute("open", "");
  await expect(souc(page).locator("details.grants-expired")).toHaveAttribute("open", "");
});
