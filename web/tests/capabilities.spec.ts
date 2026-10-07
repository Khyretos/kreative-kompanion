import { test, expect, type Page } from "@playwright/test";

async function openDemo(page: Page): Promise<void> {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await expect(page.locator("#left")).toBeVisible();
}

async function openCapabilities(page: Page): Promise<void> {
  await page.click('[data-action="capabilities"]');
  await expect(page.locator("#caps h1")).toHaveText("Capabilities");
}

test.describe("Capabilities", () => {
  test.beforeEach(async ({ page }) => {
    await openDemo(page);
  });

  test("shows models, computers, tools, MCP servers, indexes and skills as cards", async ({ page }) => {
    let loads = 0;
    page.on("load", () => { loads++; });
    await openCapabilities(page);
    await expect(page.locator(".caps-group:has(#caps-gpus) li.cap", { hasText: "a580" }).locator(".chip.state")).toHaveText("protected");
    await expect(page.locator(".caps-group:has(#caps-gpus) li.cap", { hasText: "a770" }).locator(".task-step")).toContainText("Coder 11.4 GB");
    for (const id of ["caps-gpus", "caps-models", "caps-computers", "caps-tools", "caps-mcp", "caps-indexes", "caps-skills"]) {
      await expect(page.locator(`#${id}`)).toBeVisible();
    }
    const down = page.locator("#caps li.cap", { hasText: "Ollama on soucouyant" });
    await expect(down.locator(".chip.state")).toHaveText("down");
    await expect(down.locator(".task-step")).toContainText("connection refused");
    await expect(page.locator("#caps li.cap", { hasText: "OVMS on kireserver" }).locator(".task-meta")).toContainText("orchestrator: Coder");
    // CHAT-01: each MCP server with its state and tools.
    const so = page.locator("#caps li.cap", { hasText: "Stack Overflow" });
    await expect(so.locator(".chip.state")).toHaveText("ok");
    await expect(so.locator(".task-step")).toContainText("search_stackoverflow, get_post");
    await expect(so.locator(".cap-sub")).toHaveText("3.5 million answered questions");
    const docs = page.locator("#caps li.cap", { hasText: "Developer docs" });
    await expect(docs.locator(".chip.state")).toHaveText("down");
    await expect(docs.locator(".task-step")).toContainText("401 Unauthorized");
    // The chat is hidden while the section is open; nothing reloaded.
    await expect(page.locator("main.center")).toBeHidden();
    expect(loads).toBe(0);
    await page.screenshot({ path: "test-results/capabilities.png", fullPage: true });
  });

  test("a revoked grant shows on the computer card", async ({ page }) => {
    await openCapabilities(page);
    const pc = page.locator("#caps li.cap", { hasText: "soucouyant" }).filter({ has: page.locator(".chip", { hasText: "online" }) });
    await expect(pc.locator(".task-step")).toContainText("/home/kees/projects/kompanion");
    // Revoke it from the Access tab, then come back.
    await page.click('[data-action="new-chat"]');
    await page.click('[data-action="tab"][data-tab="access"]');
    page.on("dialog", (d) => d.accept()); // "Revoke access to …?"
    const row = page.locator("li.grant-row", { hasText: "/home/kees/projects/kompanion" });
    await row.locator('[data-action="grant-revoke"]').click();
    await expect(row).toHaveCount(0);
    await openCapabilities(page);
    // ACC-01: the demo also has a system grant and an expired one, so check the revoked folder is gone.
    await expect(pc.locator(".task-step")).not.toContainText("/home/kees/projects/kompanion");
    await expect(pc.locator(".task-step")).toContainText("system (packages, root)");
  });

  test("a skill opens read-only and closes with Escape, × and an outside click", async ({ page }) => {
    await openCapabilities(page);
    const card = page.locator('#caps [data-action="open-skill"][data-id="worker/rust"]');
    const sheet = page.locator(".skill-sheet");

    await card.click();
    await expect(sheet.locator("h2")).toContainText("Skill: worker/rust");
    await expect(sheet.locator(".skill-body ol li")).toHaveCount(2);
    await expect(sheet.locator("textarea")).toHaveCount(0);
    await expect(sheet.locator(".skill-lessons li")).toHaveCount(0);
    await page.keyboard.press("Escape");
    await expect(sheet).toHaveCount(0);
    await expect(card).toBeFocused();

    await card.click();
    await sheet.locator('button[aria-label="Close"]').click();
    await expect(sheet).toHaveCount(0);

    await card.click();
    await sheet.click({ position: { x: 5, y: 5 } });
    await expect(sheet).toHaveCount(0);
  });

  test("edits a card, shows its history and moves a lesson", async ({ page }) => {
    await openCapabilities(page);
    const card = page.locator('#caps [data-action="open-skill"][data-id="work-habits"]');
    const sheet = page.locator(".skill-sheet");

    await card.click();
    await expect(sheet.locator("h2 .chip")).toHaveText("general");
    await expect(sheet.locator(".skill-lessons li")).toHaveCount(2);
    const moveBtn = sheet.locator(".skill-lessons li button").first();
    await expect(moveBtn).toHaveText("Move to private");

    await moveBtn.click();
    await expect(sheet.locator(".skill-lessons li")).toHaveCount(1);
    await expect(sheet.locator(".skill-body ol li")).toHaveCount(1);

    await sheet.getByRole("button", { name: "History" }).click();
    await expect(sheet.locator(".skill-history li")).toContainText("a1b2c3d");

    await sheet.getByRole("button", { name: "Edit" }).click();
    const textarea = sheet.locator("textarea.skill-edit");
    await expect(textarea).toBeVisible();
    await textarea.fill("# Work habits\n\n1. One lesson.\n");
    await sheet.locator("input.skill-message").fill("Shorter");
    await sheet.getByRole("button", { name: "Save" }).click();

    await expect(textarea).toHaveCount(0);
    await expect(sheet).toContainText("Saved as commit c0ffee1.");
    await expect(sheet.locator(".skill-body")).toContainText("One lesson.");

    await page.keyboard.press("Escape");
    await expect(sheet).toHaveCount(0);
  });
});
