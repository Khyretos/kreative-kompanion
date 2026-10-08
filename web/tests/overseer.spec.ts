import { test, expect, type Page } from "@playwright/test";

async function openChat(page: Page): Promise<void> {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await expect(page.locator("#left")).toBeVisible();
  await page.click("#left .new-chat");
}

test.describe("Overseer (OVR-01)", () => {
  test("chip turns it on, answers show proposals, Create makes the project and tasks", async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await openChat(page);
    const chip = page.locator("#overseer-slot .overseer-chip");
    await expect(chip).toHaveAttribute("aria-pressed", "false");
    await expect(chip).toHaveAttribute("title", /Turn on Loquendo/);
    await expect(chip).toHaveCSS("border-top-left-radius", "5px");
    await chip.click();
    await expect(chip).toHaveAttribute("aria-pressed", "true");
    await expect(chip).toContainText("Loquendo");
    // The effort chip names the Overseer role's model.
    await expect(page.locator(".effort-chip")).toContainText("qwen3.5-9b");

    await page.fill("#prompt", "How are my projects? Also plan Medabots.");
    await page.press("#prompt", "Enter");
    const answer = page.locator("article.msg.orchestrator").last();
    await expect(answer.locator(".who")).toHaveText("Loquendo");
    const box = answer.locator(".overseer-box");
    await expect(box).toBeVisible();
    // The raw TASK / INTERJECT lines are hidden; the box shows them.
    await expect(answer.locator(".body")).not.toContainText("TASK:");
    await expect(answer.locator(".body")).not.toContainText("INTERJECT:");
    await expect(box.locator(".ov-head")).toContainText("Medabots");
    await expect(box.locator(".ov-new")).toHaveText("new project");
    await expect(box.locator(".ov-tasks li")).toHaveCount(2);
    await expect(box.locator(".ov-interject .chip")).toHaveText("Added to the task");

    await box.locator('[data-action="overseer-create"]').click();
    await expect(box.locator(".ov-head .ov-done")).toHaveText("Created");
    await expect(box.locator('[data-action="overseer-create"]')).toHaveCount(0);
    await expect(page.locator("#left")).toContainText("Medabots");
    // UI-04: one compact 26px size, also at phone width.
    await page.setViewportSize({ width: 390, height: 844 });
    await expect(chip).toBeVisible();
    await expect.poll(() => chip.evaluate((el) => el.getBoundingClientRect().height)).toBe(26);
    expect(errors).toEqual([]);
  });

  test("Settings: Overseer role, name and the interject switch", async ({ page }) => {
    await openChat(page);
    await page.locator('[data-action="settings"]').first().click();
    await expect(page.locator("#role-overseer")).toBeVisible();
    await expect(page.locator("#overseer-name")).toHaveValue("Loquendo");
    await expect(page.locator("#overseer-interject")).toBeChecked();
    await page.locator("#overseer-interject").uncheck();
    await page.fill("#overseer-name", "Medabot");
    await page.locator("#overseer-name").blur();
    await page.locator('[data-action="close-settings"]').click();
    const chip = page.locator("#overseer-slot .overseer-chip");
    await expect(chip).toHaveAttribute("title", /Turn on Medabot/);
    await chip.click();
    await expect(chip).not.toHaveAttribute("title", /add context/);
  });
});
