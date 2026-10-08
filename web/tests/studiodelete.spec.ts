import { test, expect } from "@playwright/test";

// STU-D1: delete a Studio result (with a 10-second Undo), clear the failed ones, delete a selection.
test.beforeEach(async ({ page }) => {
  await page.clock.install();
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="studio"]');
  await expect(page.locator("#studio .studio-run.s-done").first()).toBeVisible();
});

test("Delete hides a result, Undo brings it back, and it is gone after 10 seconds", async ({ page }) => {
  const runs = page.locator("#studio .studio-run");
  const before = await runs.count();
  const del = page.locator('#studio .studio-run.s-done [data-action="studio-delete"]').first();
  await expect(del).toHaveAttribute("title", "Delete");
  await del.click();
  await expect(runs).toHaveCount(before - 1);
  await page.locator(".studio-undo button", { hasText: "Undo" }).click();
  await expect(runs).toHaveCount(before);
  await expect(page.locator(".studio-undo")).toHaveCount(0);
  await del.click();
  await page.clock.fastForward(10_500);
  await expect(page.locator(".studio-undo")).toHaveCount(0);
  await expect(runs).toHaveCount(before - 1);
});

test("Clear failed removes every failed result", async ({ page }) => {
  const clear = page.locator('[data-action="studio-clear-failed"]');
  await expect(clear).toHaveText("Clear failed (1)");
  await clear.click();
  await expect(page.locator("#studio .studio-run.s-failed")).toHaveCount(0);
  await expect(clear).toHaveCount(0);
  await page.clock.fastForward(10_500);
  await expect(page.locator("#studio .studio-run.s-failed")).toHaveCount(0);
});

test("Pick several and delete them from the selection bar", async ({ page }) => {
  const runs = page.locator("#studio .studio-run");
  const before = await runs.count();
  const picks = page.locator('#studio [data-action="studio-pick"]');
  await picks.nth(0).click();
  await picks.nth(1).click();
  await expect(picks.nth(0)).toHaveAttribute("aria-checked", "true");
  await expect(page.locator(".studio-lib-tools")).toContainText("2 selected");
  await page.locator('[data-action="studio-delete-picked"]').click();
  await expect(runs).toHaveCount(before - 2);
  await expect(page.locator('[data-action="studio-delete-picked"]')).toHaveCount(0);
});
