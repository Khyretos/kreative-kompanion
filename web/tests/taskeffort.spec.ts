import { test, expect } from "@playwright/test";

test.describe("Task effort", () => {
  const card = (page: import("@playwright/test").Page) =>
    page.locator(".tasks .task-main", { hasText: "Profile shader compile times" });

  test.beforeEach(async ({ page }) => {
    await page.goto("/?demo");
    await page.click("button.found-server");
    await page.click('[data-action="project"][data-id="p-kk"]');
  });

  test("a task card shows its effort", async ({ page }) => {
    await expect(card(page).locator(".task-meta")).toContainText("Auto");
  });

  test("the Start form offers the four levels, the task's own selected", async ({ page }) => {
    await card(page).click();
    const options = page.locator('form.task-run select[name="effort"] option');
    await expect(options).toHaveText(["Auto", "Low", "Medium", "High"]);
    await expect(page.locator('form.task-run select[name="effort"]')).toHaveValue("auto");
  });

  test("a run shows the effort it used", async ({ page }) => {
    await card(page).click();
    const li = page.locator("#right .task-runs li");
    await expect(li).toHaveCount(1);
    await expect(li).toContainText("High");
  });
});
