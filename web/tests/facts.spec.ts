import { test, expect, type Page } from "@playwright/test";

// FACTS: the Facts tab: project facts and chat facts in two groups, add, edit in place, delete with
// Undo, search, the flagged chip first; all without a reload; 44 px targets at 390 px in both themes.
async function openFacts(page: Page) {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.locator('#left [data-action="open-chat"]', { hasText: "Debug tooling and tests" }).click();
  await page.click('[data-action="open-facts"]');
  await expect(page.locator('[role="tab"][data-tab="facts"]')).toHaveAttribute("aria-selected", "true");
}

const group = (page: Page, scope: "project" | "chat") => page.locator(`.facts-group[data-scope="${scope}"]`);

test("the chat header opens Facts with the project's and the chat's facts, flagged first", async ({ page }) => {
  await openFacts(page);
  await expect(group(page, "project").locator("h3")).toHaveText(/Project · kk-engine\s*5/);
  await expect(group(page, "chat").locator("h3")).toHaveText(/This chat\s*2/);
  const first = group(page, "project").locator(".fact").first();
  await expect(first.locator(".fact-key")).toHaveText("Shader cache");
  await expect(first.locator(".fact-flag")).toHaveText(/May be out of date/);
  await expect(group(page, "project").locator(".fact-key")).toHaveText(["Shader cache", "Build command", "Docs", "Engine repo", "Validation default"]);
  await expect(group(page, "project").locator(".fact", { hasText: "Docs" }).locator("a.fact-value")).toHaveAttribute("href", "https://kk-engine.kreative-kompas.com/docs");
  await expect(group(page, "project").locator(".fact", { hasText: "Validation default" }).locator(".fact-meta")).toContainText("Set by Kompanion");
  await expect(group(page, "chat").locator(".fact", { hasText: "Test runner" }).locator(".fact-meta")).toContainText("Set by you");
});

test("add, edit and delete with Undo, without a reload", async ({ page }) => {
  await openFacts(page);
  const chat = group(page, "chat");
  await chat.locator('input[name="key"]').fill("Release branch");
  await chat.locator('input[name="value"]').fill("release/0.9");
  await chat.locator('select[name="kind"]').selectOption("other");
  await chat.locator('button[type="submit"]').click();
  await expect(chat.locator(".fact", { hasText: "Release branch" }).locator(".fact-value")).toHaveText("release/0.9");
  await expect(chat.locator("h3")).toHaveText(/This chat\s*3/);
  await expect(chat.locator('input[name="key"]')).toHaveValue("");
  // A duplicate name is refused with a reason.
  await chat.locator('input[name="key"]').fill("release BRANCH");
  await chat.locator('input[name="value"]').fill("x");
  await chat.locator('button[type="submit"]').click();
  await expect(page.locator(".toast[role=alert]")).toHaveText(/already a fact named/);
  // Edit an assistant fact: it becomes the user's and loses its flag.
  const project = group(page, "project");
  await project.locator('[data-action="fact-edit"][aria-label="Edit Shader cache"]').click();
  const form = project.locator("form.fact-edit");
  await form.locator('textarea[name="value"]').fill("/home/kees/.cache/kk-engine/shaders-v2");
  await form.locator('button[type="submit"]').click();
  const edited = project.locator(".fact", { hasText: "Shader cache" });
  await expect(edited.locator(".fact-value")).toHaveText("/home/kees/.cache/kk-engine/shaders-v2");
  await expect(edited.locator(".fact-flag")).toHaveCount(0);
  await expect(edited.locator(".fact-meta")).toContainText("Set by you");
  // Delete, Undo brings it back; delete again and it stays gone after the 6 s.
  await project.locator('[data-action="fact-delete"][aria-label="Delete Docs"]').click();
  await expect(project.locator(".fact", { hasText: "Docs" })).toHaveCount(0);
  await page.locator(".toast.fact-undo button").click();
  await expect(project.locator(".fact", { hasText: "Docs" })).toHaveCount(1);
  await project.locator('[data-action="fact-delete"][aria-label="Delete Docs"]').click();
  await expect(page.locator(".toast.fact-undo")).toBeVisible();
  await expect(page.locator(".toast.fact-undo")).toHaveCount(0, { timeout: 9_000 });
  await expect(project.locator(".fact", { hasText: "Docs" })).toHaveCount(0);
  await expect(project.locator("h3")).toHaveText(/4/);
});

test("search filters both groups by name and value", async ({ page }) => {
  await openFacts(page);
  await page.fill("#facts-q", "kk-engine");
  await expect(page.locator(".fact .fact-key")).toHaveText(["Shader cache", "Docs", "Engine repo"]);
  await expect(group(page, "chat").locator(".facts-empty")).toHaveText("No fact matches your search.");
  await expect(page.locator("#facts-q")).toBeFocused();
  await page.fill("#facts-q", "");
  await expect(page.locator(".fact")).toHaveCount(7);
});

test("a token is refused with where-it-lives advice", async ({ page }) => {
  await openFacts(page);
  const chat = group(page, "chat");
  await chat.locator('input[name="key"]').fill("Forgejo");
  await chat.locator('input[name="value"]').fill("ghp_" + "Ab3dEf7h".repeat(3));
  await chat.locator('button[type="submit"]').click();
  await expect(page.locator(".toast[role=alert]")).toHaveText(/never hold secrets: write where it lives/);
  await expect(chat.locator(".fact", { hasText: "Forgejo" })).toHaveCount(0);
});

for (const scheme of ["light", "dark"] as const) {
  test(`at 390 px Facts fits the phone with 44 px targets (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await openFacts(page);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.waitForFunction(() => document.getAnimations().length === 0);
    await expect(group(page, "project")).toBeVisible();
    for (const b of await page.locator(".fact-buttons .icon-btn").all()) {
      const box = (await b.boundingBox())!;
      expect(box.height).toBeGreaterThanOrEqual(44);
      expect(box.width).toBeGreaterThanOrEqual(44);
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
    const right = (await page.locator(".facts-pane").boundingBox())!;
    expect(right.x + right.width).toBeLessThanOrEqual(391);
  });
}

test("Remember facts is a Tools menu entry that turns on per chat", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.locator('#left [data-action="open-chat"]').first().click();
  await page.locator(".tools-chip").click();
  const item = page.locator('.tools-menu [role="menuitemcheckbox"]', { hasText: "Remember facts" });
  await expect(item.locator(".effort-hint")).toHaveText(/save facts worth keeping/);
  await item.click();
  await expect(item).toHaveAttribute("aria-checked", "true");
});

test("at 390 px the five tabs stay on one line with the close button", async ({ page }) => {
  await openFacts(page);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.waitForFunction(() => document.getAnimations().length === 0);
  const head = page.locator(".right .pane-head");
  const close = head.locator('[data-action="pane"][data-pane="main"]');
  await expect(close).toBeVisible();
  const [t, c] = [(await head.locator(".tabs").boundingBox())!, (await close.boundingBox())!];
  expect(Math.abs(t.y + t.height / 2 - (c.y + c.height / 2))).toBeLessThan(12);
  expect(c.x + c.width).toBeLessThanOrEqual(390);
  // The Facts tab is scrolled into view.
  const tab = (await head.locator('[data-tab="facts"]').boundingBox())!;
  expect(tab.x + tab.width).toBeLessThanOrEqual(c.x + 1);
});
