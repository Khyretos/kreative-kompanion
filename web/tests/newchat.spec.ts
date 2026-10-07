import { test, expect, type Page } from "@playwright/test";

async function openDemo(page: Page): Promise<void> {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await expect(page.locator("#left")).toBeVisible();
}

test.describe("New chat without a project", () => {
  test("New chat on a fresh start sends the first message", async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await openDemo(page);
    await page.click('#left .new-chat');
    await page.fill("#prompt", "Hello without a project");
    await page.press("#prompt", "Enter");
    await expect(page.locator("#conv-head .conv-title h1")).toHaveText("Hello without a project");
    await expect(page.locator("#conv-head .conv-title .eyebrow")).toHaveText("Chat");
    expect(errors).toEqual([]);
  });

  test("New chat after opening a project starts a chat without that project", async ({ page }) => {
    await openDemo(page);
    await page.click('[data-action="project"][data-id="p-kk"]');
    await page.click('#left .new-chat');
    await expect(page.locator("#conv-head .conv-title .eyebrow")).toHaveText("Chat");
    await page.fill("#prompt", "Loose question");
    await page.press("#prompt", "Enter");
    await expect(page.locator("#conv-head .conv-title h1")).toHaveText("Loose question");
    await expect(page.locator("#conv-head .conv-title .eyebrow")).toHaveText("Chat");
  });

  test("New chat here inside a project still starts a chat in that project", async ({ page }) => {
    await openDemo(page);
    await page.click('[data-action="project"][data-id="p-kk"]');
    await page.click('#left .new-in-project[data-project="p-kk"]');
    await expect(page.locator("#conv-head .conv-title .eyebrow")).toHaveText("kk-engine");
    await page.fill("#prompt", "Project question");
    await page.press("#prompt", "Enter");
    await expect(page.locator("#conv-head .conv-title h1")).toHaveText("Project question");
    await expect(page.locator("#conv-head .conv-title .eyebrow")).toHaveText("kk-engine");
  });
});
