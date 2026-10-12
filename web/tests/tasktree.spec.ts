// KG-01: a goal with milestones, tasks and dependencies is stored, shown as a tree on the
// project's Tasks pane (title + badges, details on click) and editable.
import { test, expect, type Page } from "@playwright/test";


async function openProject(page: Page): Promise<void> {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="project"][data-id="p-kk"]');
  await expect(page.locator("#project-type")).toBeVisible();
}

/** Adds a task through the editor and goes back to the list. */
async function add(page: Page, title: string, kind: string, parent?: string, deps: string[] = []): Promise<void> {
  await page.click('[data-action="new-task"]');
  await page.fill("#task-title", title);
  await page.fill("#task-goal", "a part of the release.");
  await page.fill("#task-steps", "Do it.");
  await page.fill("#task-done", "it works.");
  await page.selectOption("#task-kind", kind);
  if (parent) await page.selectOption("#task-parent", { label: parent });
  for (const d of deps) {
    await page.fill("#dep-search", d);
    await page.locator("#dep-options [role=option]:visible", { hasText: d }).click();
  }
  await page.click('#task-editor button[type="submit"]');
  await expect(page.locator(".task-detail h3")).toHaveText(title);
  await page.click('[data-action="close-task"]');
}

const row = (page: Page, title: string) => page.locator(".task-tree .tree-row", { has: page.locator(".tree-title", { hasText: title }) });
const node = (page: Page, title: string) => page.locator(".task-tree li.tree-node", { has: page.locator(":scope > .tree-row .tree-title", { hasText: title }) });

async function buildTree(page: Page): Promise<void> {
  await add(page, "Release 1.0", "goal");
  await add(page, "Engine milestone", "milestone", "Release 1.0");
  await add(page, "Write docs", "task", "Engine milestone");
  await add(page, "Ship build", "task", "Engine milestone", ["Write docs"]);
}

test("a goal with a milestone, tasks and a dependency shows as a tree", async ({ page }) => {
  await openProject(page);
  await expect(page.locator(".task-tree")).toHaveCount(0); // no tree data yet: no Plan section
  await buildTree(page);
  const tree = page.locator('section.task-tree[aria-label="Plan"]');
  await expect(tree).toBeVisible();
  // Nesting: goal > milestone > two tasks.
  await expect(node(page, "Release 1.0").locator("ul.tree-children li.tree-node", { hasText: "Engine milestone" })).toHaveCount(1);
  await expect(node(page, "Engine milestone").locator(":scope > ul.tree-children > li.tree-node")).toHaveCount(2);
  // Badges: kind on goals and milestones, state everywhere, progress on parents, waits on undone deps.
  await expect(row(page, "Release 1.0").locator(".chip.kind")).toHaveText("Goal");
  await expect(row(page, "Engine milestone").locator(".chip.kind")).toHaveText("Milestone");
  await expect(row(page, "Write docs").locator(".chip.kind")).toHaveCount(0);
  await expect(row(page, "Write docs").locator(".chip.state")).toHaveText("Queued");
  await expect(row(page, "Engine milestone").locator(".tree-count")).toHaveText("0/2 done");
  await expect(row(page, "Ship build").locator(".chip.waits")).toHaveText("Waits on 1");
  await expect(row(page, "Write docs").locator(".chip.waits")).toHaveCount(0);

  // Details on click: where it belongs and what it waits for, as links.
  await row(page, "Ship build").click();
  const links = page.locator(".task-detail .task-links");
  await expect(links.locator('[data-action="open-task"]', { hasText: "Engine milestone" })).toBeVisible();
  await expect(links.locator('[data-action="open-task"]', { hasText: "Write docs" })).toBeVisible();
  await links.locator('[data-action="open-task"]', { hasText: "Write docs" }).click();
  await expect(page.locator(".task-detail h3")).toHaveText("Write docs");
  await page.click('[data-action="close-task-done"]');
  await page.click('[data-action="close-task"]');
  await expect(row(page, "Ship build").locator(".chip.waits")).toHaveCount(0);
  await expect(row(page, "Engine milestone").locator(".tree-count")).toHaveText("1/2 done");
});

test("the tree is editable: move, drop a dependency, delete a milestone", async ({ page }) => {
  await openProject(page);
  await buildTree(page);
  // The editor shows the current tree fields; a goal offers no parent.
  await row(page, "Ship build").click();
  await page.click('.task-detail [data-action="edit-task"]');
  await expect(page.locator("#task-kind")).toHaveValue("task");
  await expect(page.locator("#task-parent option:checked")).toHaveText("Engine milestone");
  await expect(page.locator(".dep-chips .dep-chip:visible")).toHaveText(["Write docs"]);
  // A task can't depend on itself or sit under itself.
  await page.fill("#dep-search", "Ship build");
  await expect(page.locator("#dep-options [role=option]:visible")).toHaveCount(0);
  await expect(page.locator("#task-parent option", { hasText: "Ship build" })).toHaveCount(0);
  await page.selectOption("#task-parent", "");
  await page.locator('.dep-chip [aria-label="Remove Write docs"]').click();
  await page.click('#task-editor button[type="submit"]');
  await page.click('[data-action="close-task"]');
  await expect(page.locator(".task-tree > ul.tree > li.tree-node", { hasText: "Ship build" })).toHaveCount(1);
  await expect(row(page, "Ship build").locator(".chip.waits")).toHaveCount(0);
  // A goal can't have a parent.
  await row(page, "Release 1.0").click();
  await page.click('.task-detail [data-action="edit-task"]');
  await expect(page.locator("#task-parent")).toBeDisabled();
  await page.click('#task-editor [data-action="cancel-task-edit"]');
  await page.click('[data-action="close-task"]');
  // Deleting the milestone moves its task up to the goal.
  page.on("dialog", (d) => d.accept());
  await row(page, "Engine milestone").click();
  await page.click('.task-detail [data-action="delete-task"]');
  await expect(node(page, "Release 1.0").locator(":scope > ul.tree-children > li.tree-node", { hasText: "Write docs" })).toHaveCount(1);
});

for (const scheme of ["light", "dark"] as const) {
  test(`the tree fits a 390 px phone in ${scheme} mode`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await openProject(page);
    await buildTree(page);
    // On a phone the Tasks pane opens from the chat header.
    await page.setViewportSize({ width: 390, height: 844 });
    await page.click('#conv-head [data-action="pane"][data-pane="right"]');
    const tree = page.locator(".task-tree");
    await expect(tree).toBeVisible();
    expect(await tree.evaluate((el) => el.scrollWidth <= el.clientWidth + 1)).toBe(true);
    await expect(async () => {
      await page.locator(".task-tree").screenshot({ path: `test-results/kg01-tree-${scheme}-390.png` });
    }).toPass({ timeout: 5000 });
  });
}
