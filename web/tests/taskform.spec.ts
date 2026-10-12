// TASK-FORM: the New task form. The description is written in guided fields (Goal, Steps, Done when,
// and a Markdown drawer for anything else) that compose the markdown; Depends on is a searchable picker
// with chips; the selects use the same purple input style as the text fields. Phone width, both themes.
import { test, expect, type Page } from "@playwright/test";

async function openProject(page: Page, width = 1280): Promise<void> {
  await page.setViewportSize({ width, height: 900 });
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.locator('[data-action="project"][data-id="p-kk"]').evaluate((el) => (el as HTMLElement).click());
  await expect(page.locator("#project-type")).toBeAttached();
}

async function newTask(page: Page): Promise<void> {
  await page.locator('[data-action="new-task"]').first().evaluate((el) => (el as HTMLElement).click());
  await expect(page.locator("#task-editor")).toBeVisible();
}

test("the description is composed from Goal, Steps and Done when", async ({ page }) => {
  await openProject(page);
  await newTask(page);
  // Guided fields, and no literal template text to delete first.
  await expect(page.locator("#task-goal")).toHaveValue("");
  await expect(page.locator("#task-steps")).toHaveValue("");
  await expect(page.locator("#task-done")).toHaveValue("");
  const texts = await page.locator("#task-editor textarea").evaluateAll((els) => els.map((e) => (e as HTMLTextAreaElement).value));
  expect(texts.join("")).not.toContain("**Goal");
  await expect(page.locator('#task-editor [name="description"]')).toHaveCount(0);
  await page.fill("#task-title", "Ship the demo build");
  await page.fill("#task-goal", "A build people can try.");
  await page.fill("#task-steps", "Write the docs\n2. Run the tests\n- Upload it");
  await page.fill("#task-done", "The download link works.");
  await page.click('#task-editor button[type="submit"]');
  const desc = page.locator(".task-detail .task-desc");
  await expect(desc).toContainText("A build people can try.");
  await expect(desc.locator("ol li")).toHaveText(["Write the docs", "Run the tests", "Upload it"]);
  await expect(desc).toContainText("The download link works.");
  // Editing splits the markdown back into the same fields.
  await page.click('.task-detail [data-action="edit-task"]');
  await expect(page.locator("#task-goal")).toHaveValue("A build people can try.");
  await expect(page.locator("#task-steps")).toHaveValue("Write the docs\nRun the tests\nUpload it");
  await expect(page.locator("#task-done")).toHaveValue("The download link works.");
  await expect(page.locator("#task-notes")).toHaveValue("");
});

test("anything else goes in the Markdown drawer and comes back there", async ({ page }) => {
  await openProject(page);
  await newTask(page);
  const drawer = page.locator("details.task-notes");
  await expect(drawer).not.toHaveAttribute("open", "");
  await drawer.locator("summary").click();
  await page.fill("#task-title", "Profile the loader");
  await page.fill("#task-goal", "Know where the time goes.");
  await page.fill("#task-done", "A table of the slowest files.");
  await page.fill("#task-notes", "Use **Tracy**, not printf.");
  await page.click('#task-editor button[type="submit"]');
  await expect(page.locator(".task-detail .task-desc strong", { hasText: "Tracy" })).toBeVisible();
  await page.click('.task-detail [data-action="edit-task"]');
  await expect(page.locator("details.task-notes")).toHaveAttribute("open", "");
  await expect(page.locator("#task-notes")).toHaveValue("Use **Tracy**, not printf.");
  // An older one-line description fills the Goal field.
  await page.click('#task-editor [data-action="cancel-task-edit"]');
  await page.click('[data-action="close-task"]');
  await page.locator('[data-action="open-task"]', { hasText: "Profile shader compile times" }).first().click();
  await page.click('.task-detail [data-action="edit-task"]');
  await expect(page.locator("#task-goal")).toHaveValue("Profile shader compile times.");
  await expect(page.locator("#task-notes")).toHaveValue("");
});

test("Depends on is a searchable picker with chips", async ({ page }) => {
  await openProject(page);
  await newTask(page);
  const search = page.locator("#dep-search");
  await expect(search).toHaveAttribute("role", "combobox");
  await search.fill("readme");
  const options = page.locator("#dep-options [role=option]:visible");
  await expect(options).toHaveText(["Update the README build steps"]);
  await options.first().click();
  const chips = page.locator(".dep-chips .dep-chip:visible");
  await expect(chips).toHaveText(["Update the README build steps"]);
  await expect(search).toHaveValue("");
  await expect(search).toBeFocused();
  // A chosen task is not offered again; Enter picks the first match and does not submit the form.
  await search.fill("update the");
  await expect(options).toHaveCount(0);
  await expect(page.locator("#dep-options .dep-none")).toBeVisible();
  await search.fill("profile");
  await search.press("Enter");
  await expect(chips).toHaveText(["Update the README build steps", "Profile shader compile times"]);
  await expect(page.locator("#task-editor")).toBeVisible();
  // Remove a chip with its own button.
  await page.locator('.dep-chip [aria-label="Remove Update the README build steps"]').click();
  await expect(chips).toHaveText(["Profile shader compile times"]);
  // Saved: the task waits on the chosen one.
  await page.fill("#task-title", "Ship the profiler");
  await page.fill("#task-goal", "Profiling in the release.");
  await page.fill("#task-done", "It ships.");
  await page.click('#task-editor button[type="submit"]');
  await expect(page.locator(".task-detail .task-links", { hasText: "Profile shader compile times" })).toBeVisible();
  await page.click('.task-detail [data-action="edit-task"]');
  await expect(page.locator(".dep-chips .dep-chip:visible")).toHaveText(["Profile shader compile times"]);
  // A task never depends on itself.
  await page.fill("#dep-search", "ship the profiler");
  await expect(page.locator("#dep-options [role=option]:visible")).toHaveCount(0);
});

for (const scheme of ["light", "dark"] as const) {
  test(`the form fits a 390 px phone with purple selects (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await openProject(page, 390);
    await newTask(page);
    await page.fill("#dep-search", "e");
    const style = await page.evaluate(() => {
      const bg = (sel: string) => getComputedStyle(document.querySelector(sel) as Element).backgroundColor;
      const sel = document.querySelector("#task-kind") as HTMLSelectElement;
      return {
        input: bg("#task-title"),
        selects: ["#task-kind", "#task-parent", "#task-state"].map(bg),
        option: getComputedStyle(sel.options[0]).backgroundColor,
        appearance: getComputedStyle(sel).appearance,
        search: bg("#dep-search"),
        form: (() => { const f = document.querySelector("#task-editor") as HTMLElement; return f.scrollWidth - f.clientWidth; })(),
      };
    });
    expect(style.selects).toEqual([style.input, style.input, style.input]);
    expect(style.option).toBe(style.input);
    expect(style.search).toBe(style.input);
    expect(style.appearance).toBe("none");
    expect(style.form).toBeLessThanOrEqual(0); // nothing sticks out sideways
    await page.locator("#task-editor").screenshot({ path: `test-results/taskform-390-${scheme}.png` });
  });
}
