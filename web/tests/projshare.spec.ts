// PROJECT-SHARING slice 2: a project shared with you shows "shared by <owner>" in the sidebar, its Overview
// has a members row (the owner adds and removes people, a member can only leave), a viewer gets a read-only
// UI (no edit, no Add task, no Settings, a disabled composer in its chats). Phone width and both themes.
import { test, expect, type Page } from "@playwright/test";

async function openDemo(page: Page, width = 1280, height = 900, scheme: "light" | "dark" = "dark"): Promise<void> {
  await page.emulateMedia({ colorScheme: scheme });
  await page.setViewportSize({ width, height });
  await page.goto("/?demo&shared");
  await page.click("button.found-server");
  await expect(page.locator("#left .project-head").first()).toBeAttached();
}

const head = (page: Page, id: string) => page.locator(`#left [data-action="project"][data-id="${id}"]`);
const overview = (page: Page) => page.locator("#right section.overview");
const row = (page: Page) => overview(page).locator(".ov-members");

for (const scheme of ["dark", "light"] as const) {
  test(`the sidebar says who shared a project (${scheme})`, async ({ page }) => {
    await openDemo(page, 1280, 900, scheme);
    await expect(head(page, "p-shared").locator(".shared-chip")).toHaveText("shared by mara");
    await expect(head(page, "p-3dco").locator(".shared-chip")).toHaveCount(0);
    await expect(page.locator("#left .shared-chip")).toHaveCount(1);
  });
}

test("a viewer sees the members and a read-only Overview, and can leave", async ({ page }) => {
  await openDemo(page);
  await head(page, "p-shared").click();
  await expect(overview(page)).toBeVisible();
  await expect(row(page)).toBeVisible();
  await expect(row(page).locator('[data-member="owner"]')).toContainText("mara");
  await expect(row(page).locator('[data-member="owner"]')).toContainText("Owner");
  await expect(row(page).locator('[data-member="u-me"]')).toContainText("Viewer");
  await expect(row(page).locator('[data-member="u-me"]')).toContainText("you");
  // Read-only: nothing to edit, add or configure.
  await expect(row(page).locator("form.member-add")).toHaveCount(0);
  await expect(row(page).locator('[data-action="member-remove"]')).toHaveCount(1);
  await expect(overview(page).locator('[data-action="overview-edit"]')).toHaveCount(0);
  await expect(page.locator('#right [data-action="new-task"]')).toHaveCount(0);
  await expect(overview(page).locator('.ov-sec[data-sec="settings"]')).toHaveCount(0);
  // Leaving removes the project from the sidebar at once.
  await row(page).locator('[data-action="member-remove"][data-user="u-me"]').click();
  await expect(head(page, "p-shared")).toHaveCount(0);
});

test("a viewer reads the chats of a shared project but cannot write", async ({ page }) => {
  await openDemo(page);
  await head(page, "p-shared").click();
  await page.locator('#left [data-action="open-chat"][data-id="c-shared"]').first().click();
  await expect(page.locator("#messages")).toContainText("Which plants sell best?");
  await expect(page.locator("#prompt")).toBeDisabled();
  await expect(page.locator("#composer")).toContainText(/read-only/i);
});

test("the owner adds a member, sees the role, and removes them again", async ({ page }) => {
  await openDemo(page);
  await head(page, "p-3dco").click();
  await expect(row(page)).toBeVisible();
  await expect(row(page).locator('[data-member="owner"]')).toContainText("Owner");
  await expect(row(page).locator("[data-member]")).toHaveCount(1);
  const form = row(page).locator("form.member-add");
  const mara = row(page).locator('[data-member="u-mara"]');
  // The Overview re-renders when late data arrives, which can drop what was typed: retry the whole step.
  await expect(async () => {
    await form.locator('[name="member"]').fill("mara");
    await form.locator('[name="role"]').selectOption("editor");
    await form.locator('button[type="submit"]').click();
    await expect(mara).toContainText("mara", { timeout: 1500 });
  }).toPass();
  await expect(mara).toContainText("Editor");
  await mara.locator('[data-action="member-remove"]').click();
  await expect(row(page).locator('[data-member="u-mara"]')).toHaveCount(0);
});

test("an unknown name is refused with the server's message and adds nobody", async ({ page }) => {
  await openDemo(page);
  await head(page, "p-3dco").click();
  const form = row(page).locator("form.member-add");
  await expect(async () => {
    await form.locator('[name="member"]').fill("nobody");
    await form.locator('button[type="submit"]').click();
    await expect(row(page).locator(".member-error")).toHaveText("No user with that name.", { timeout: 1500 });
  }).toPass();
  await expect(row(page).locator("[data-member]")).toHaveCount(1);
});

test("at 390 px the members row fits and nothing scrolls sideways", async ({ page }) => {
  await openDemo(page, 1280, 900, "light");
  await page.setViewportSize({ width: 390, height: 844 });
  await page.locator('[data-action="pane"][data-pane="left"]').first().click();
  await head(page, "p-3dco").click();
  await expect(page.locator(".shell")).toHaveAttribute("data-pane", "right");
  await expect(row(page)).toBeVisible();
  const wide = await page.evaluate(() => {
    const right = document.querySelector("#right") as HTMLElement;
    return { doc: document.documentElement.scrollWidth, right: right.scrollWidth - right.clientWidth };
  });
  expect(wide.doc).toBeLessThanOrEqual(390);
  expect(wide.right).toBeLessThanOrEqual(0);
  await page.screenshot({ path: "test-results/projshare-390-light.png", fullPage: true });
});
