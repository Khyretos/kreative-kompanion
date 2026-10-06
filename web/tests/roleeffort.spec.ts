import { test, expect } from "@playwright/test";

test.describe("Role effort", () => {
  const card = (page: import("@playwright/test").Page) =>
    page.locator(".tasks .task-main", { hasText: "Profile shader compile times" });

  test.beforeEach(async ({ page }) => {
    await page.goto("/?demo");
    await page.click("button.found-server");
    await page.click('[data-action="project"][data-id="p-kk"]');
  });

  test("each role offers the four levels, Auto by default", async ({ page }) => {
    await page.click('[data-action="settings"]');
    const options = page.locator('#role-effort-worker option');
    await expect(options).toHaveText(["Auto", "Low", "Medium", "High"]);
    await expect(page.locator("#role-effort-worker")).toHaveValue("auto");
    await expect(page.locator("#role-effort-orchestrator")).toBeVisible();
    await expect(page.locator("#role-effort-reviewer")).toBeVisible();
  });

  test("a role's effort is kept and the model stays", async ({ page }) => {
    await page.click('[data-action="settings"]');
    const model = await page.locator("#role-worker").inputValue();
    expect(model).not.toBe("");
    await page.selectOption("#role-effort-worker", "high");
    await expect(page.locator("#role-effort-worker")).toHaveValue("high");
    await expect(page.locator("#role-worker")).toHaveValue(model);
    await page.keyboard.press("Escape");
    await page.click('[data-action="settings"]');
    await expect(page.locator("#role-effort-worker")).toHaveValue("high");
  });

  test("a run Auto picked says so", async ({ page }) => {
    await card(page).click();
    await expect(page.locator("#right .task-runs li")).toContainText("High (Auto picked)");
  });
});
