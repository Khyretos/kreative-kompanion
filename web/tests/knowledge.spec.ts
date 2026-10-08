import { test, expect, type Page } from "@playwright/test";

// CHAT-03b (Kees, 2026-10-07): knowledge collections in the app: upload files, link them to
// projects, see how far search by meaning is. Title and badges at a glance, the rest in Details.
async function openCaps(page: Page): Promise<void> {
  await page.goto("/?demo");
  await page.click("button.found-server");
  if ((page.viewportSize()?.width ?? 1280) < 760) await page.click('#conv-head [data-action="pane"][data-pane="left"]');
  await page.click('[data-action="capabilities"]');
  await expect(page.locator("#caps .kn-card").first()).toBeVisible();
}

const card = (page: Page, name: string) => page.locator("#caps .kn-card").filter({ has: page.locator(".cap-title", { hasText: name }) });

test("collections show their source, size, meaning search and linked projects at a glance", async ({ page }) => {
  await openCaps(page);
  await expect(page.locator("#caps-knowledge")).toHaveText(/Knowledge/);
  const godot = card(page, "Godot Assistant");
  await expect(godot.locator(".chip.state")).toHaveText("Open WebUI");
  await expect(godot.locator(".task-step")).toContainText("412 documents");
  await expect(godot.locator(".task-step")).toContainText("search by meaning 66%");
  const notes = card(page, "Game notes");
  await expect(notes.locator(".chip.state")).toHaveText("uploaded");
  await expect(notes.locator(".task-step")).toContainText("search by meaning ready");
  await expect(notes.locator(".kn-projects .chip")).toHaveText(["kk-engine"]);
  // Details stay closed until asked for.
  await expect(notes.locator("details.kn-more")).not.toHaveAttribute("open", "");
  // The knowledge collections are not listed twice under the asset indexes.
  await expect(page.locator("#caps .caps-group").filter({ has: page.locator("#caps-indexes") }).locator(".cap-title", { hasText: "Godot Assistant" })).toHaveCount(0);
});

test("a new collection takes uploaded files and shows them in its details", async ({ page }) => {
  await openCaps(page);
  await page.fill("#kn-name", "Design docs");
  await page.click("form.kn-new button[type=submit]");
  const c = card(page, "Design docs");
  await expect(c).toBeVisible();
  await c.locator("details.kn-more > summary").click();
  await c.locator("input.kn-file").setInputFiles([
    { name: "combat.md", mimeType: "text/markdown", buffer: Buffer.from("# Combat\n\nParry window is 200 ms.") },
    { name: "song.mp3", mimeType: "audio/mpeg", buffer: Buffer.from("ID3") },
  ]);
  await expect(c.locator(".kn-docs li", { hasText: "combat.md" })).toBeVisible();
  await expect(c.locator(".task-step")).toContainText("1 documents");
  // A file the server refuses says why, in the card.
  await expect(c.locator(".kn-up.failed", { hasText: "song.mp3" }).locator(".kn-err")).toContainText("unsupported file type");
  // The details stay open across the live refresh.
  await expect(c.locator("details.kn-more")).toHaveAttribute("open", "");
  // Remove the document again.
  await c.locator('.kn-docs [data-action="kn-doc-delete"]').first().click();
  await expect(c.locator(".kn-docs li", { hasText: "combat.md" })).toHaveCount(0);
});

test("linking a collection to a project shows the project on the card", async ({ page }) => {
  await openCaps(page);
  const godot = card(page, "Godot Assistant");
  await godot.locator("details.kn-more > summary").click();
  // Open WebUI's collections are filled from there: no upload field.
  await expect(godot.locator("input.kn-file")).toHaveCount(0);
  await expect(godot.locator(".kn-synced")).toBeVisible();
  await godot.locator('input.kn-project[value="p-kk"]').check();
  await expect(godot.locator(".kn-projects .chip")).toHaveText(["kk-engine"]);
  await godot.locator('input.kn-project[value="p-kk"]').uncheck();
  await expect(godot.locator(".kn-projects .chip")).toHaveCount(0);
});

test("deleting a collection asks first", async ({ page }) => {
  await openCaps(page);
  const notes = card(page, "Game notes");
  await notes.locator("details.kn-more > summary").click();
  page.once("dialog", (d) => void d.accept());
  await notes.locator('[data-action="kn-delete"]').click();
  await expect(card(page, "Game notes")).toHaveCount(0);
});

for (const width of [375, 1280]) {
  test(`knowledge cards do not overflow at ${width} px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await openCaps(page);
    await card(page, "Game notes").locator("details.kn-more > summary").click();
    const wide = await page.evaluate(() => [...document.querySelectorAll<HTMLElement>("#caps .kn-card, #caps .kn-card *, #caps form.kn-new")]
      .filter((el) => el.offsetParent !== null && !el.classList.contains("sr-only") && el.scrollWidth > el.clientWidth + 1 && el.clientWidth > 0)
      .map((el) => `${el.className || el.tagName} (${el.scrollWidth} > ${el.clientWidth})`));
    expect(wide).toEqual([]);
  });
}

// KNOW-01 (Kees, 2026-10-08): the documents in a collection can be opened and read, not only listed.
test("a collection's documents open in a reader with a name filter", async ({ page }) => {
  await openCaps(page);
  const c = card(page, "Game notes");
  await c.locator("details.kn-more > summary").click();
  await c.getByRole("button", { name: "Read documents" }).click();
  const reader = page.locator("dialog.kn-reader");
  await expect(reader.locator(".kn-reader-doc")).toHaveText(["combat-design.md", "level-ideas.pdf"]);
  await reader.locator(".kn-reader-search").fill("level");
  await expect(reader.locator(".kn-reader-doc")).toHaveText(["level-ideas.pdf"]);
  await reader.locator(".kn-reader-doc").click();
  await expect(reader.locator(".kn-reader-name")).toHaveText("level-ideas.pdf");
  await expect(reader.locator(".kn-reader-text")).toContainText("The text of this document");
  await page.keyboard.press("Escape");
  await expect(reader).toHaveCount(0);
});
