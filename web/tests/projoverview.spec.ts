// PROJ-OVERVIEW: clicking a project opens its Overview at the top of the Tasks pane: what it is, where it
// lives, what it may reach, how far along it is, the last activity and the models used. The owner edits
// name, description, about and links in place (no reload, the sidebar follows at once). Phone width and
// both themes; tiles wrap to 2x2 at 390 px and nothing scrolls sideways.
import { test, expect, type Page } from "@playwright/test";

async function openDemo(page: Page, width = 1280, height = 900, scheme: "light" | "dark" = "dark"): Promise<void> {
  await page.emulateMedia({ colorScheme: scheme });
  await page.setViewportSize({ width, height });
  await page.goto("/?demo");
  await page.click("button.found-server");
  await expect(page.locator("#left .project-head").first()).toBeAttached();
}

const head = (page: Page, id: string) => page.locator(`#left [data-action="project"][data-id="${id}"]`);
const overview = (page: Page) => page.locator("#right section.overview");
const sec = (page: Page, name: string) => overview(page).locator(`.ov-sec[data-sec="${name}"]`);
const secToggle = (page: Page, name: string) => sec(page, name).locator(`[data-action="overview-sec"]`);

/** kk-engine as a programming project on soucouyant with the game preset's commands, set up in Settings. */
async function setUpKk(page: Page): Promise<void> {
  await head(page, "p-kk").click();
  await secToggle(page, "settings").click();
  await page.selectOption("#project-type", "programming");
  const repo = page.locator("#right form.project-repo");
  await repo.locator('select[name="machine"]').selectOption({ label: "soucouyant" });
  await repo.locator('input[name="folder"]').fill("/home/kees/projects/kompanion/kk-engine");
  await repo.locator('button[type="submit"]').click();
  // Each save sends a change event that re-renders the panel: wait for it before the next form.
  await expect(sec(page, "where")).toContainText("soucouyant");
  const dev = page.locator("#right form.project-dev");
  // A late change event can re-render the form between the preset and Save; then save again.
  await expect(async () => {
    await dev.locator("#dev-preset").selectOption("game");
    await expect(dev.locator('[name="build"]')).toHaveValue("cmake --build build");
    await dev.locator('button[type="submit"]').click();
    await expect(sec(page, "build")).toContainText("cmake --build build", { timeout: 2000 });
  }).toPass({ timeout: 15_000 });
  await secToggle(page, "settings").click();
  await expect(page.locator("#right .project-panel")).toHaveCount(0);
}

for (const scheme of ["dark", "light"] as const) {
  test(`clicking a project shows its overview (desktop, ${scheme})`, async ({ page }) => {
    await openDemo(page, 1280, 900, scheme);
    await setUpKk(page);
    const o = overview(page);
    await expect(o).toHaveAttribute("data-id", "p-kk");
    await expect(o.locator(".ov-name")).toHaveText("kk-engine");
    await expect(o.locator(".ov-desc")).toHaveText("C++17 Vulkan game engine");
    await expect(o.locator(".ov-chips .chip").first()).toHaveText("Programming");
    // Four glance tiles.
    const tiles = o.locator(".ov-tiles .ov-tile");
    await expect(tiles).toHaveCount(4);
    await expect(o.locator('.ov-tile[data-tile="progress"] .ov-num')).toHaveText(/^\d+\/\d+$/);
    // kk-engine has nothing waiting for Kees, so the tile stays calm (orange only when > 0).
    await expect(o.locator('.ov-tile[data-tile="needs"] .ov-num')).toHaveText("0");
    await expect(o.locator('.ov-tile[data-tile="needs"]')).not.toHaveClass(/attn/);
    await expect(o.locator('.ov-tile[data-tile="activity"] .ov-num')).not.toHaveText("");
    // About: clamped with "Show more", which opens the rest.
    const about = o.locator(".ov-about");
    await expect(about).toContainText("A small C++17 engine");
    await expect(about).toHaveClass(/clamped/);
    await o.locator('[data-action="overview-more"]').click();
    await expect(about).not.toHaveClass(/clamped/);
    await expect(o.locator('[data-action="overview-more"]')).toHaveText("Show less");
    // Where it lives, access, build and test, activity and AI use are open on a desktop.
    await expect(sec(page, "where")).toContainText("soucouyant");
    await expect(sec(page, "where").locator(".ov-folder")).toHaveText("/home/kees/projects/kompanion/kk-engine");
    await expect(sec(page, "where").locator('a.chip[href="https://git.kreative-kompas.com/khyretos/kk-engine"]')).toHaveText("Repo");
    await expect(sec(page, "access")).toContainText("Read and write");
    await expect(sec(page, "build")).toContainText("cmake --build build");
    await expect(sec(page, "activity").locator(".ov-act li")).toHaveCount(5);
    await sec(page, "activity").locator('[data-action="overview-activity-more"]').click();
    await expect(sec(page, "activity").locator(".ov-act li")).toHaveCount(10);
    await expect(sec(page, "ai")).toContainText("Qwen3.5-9B");
    // The settings forms sit in a collapsed Settings section below.
    await expect(secToggle(page, "settings")).toHaveAttribute("aria-expanded", "false");
    await expect(page.locator("#right .project-panel")).toHaveCount(0);
    await secToggle(page, "settings").click();
    await expect(page.locator("#right .project-panel")).toBeVisible();
    // The overview is above the task list.
    await expect.poll(async () => {
      const ob = await o.boundingBox();
      const bar = await page.locator("#right .project-bar").boundingBox();
      return !!ob && !!bar && ob.y < bar.y;
    }).toBe(true);
    await page.screenshot({ path: `test-results/overview-1280-${scheme}.png`, fullPage: true });
  });
}

test("edit name, description, about and links in place: no reload, the sidebar follows", async ({ page }) => {
  await openDemo(page);
  let loads = 0;
  page.on("load", () => { loads++; });
  await setUpKk(page);
  await overview(page).locator('[data-action="overview-edit"]').click();
  const form = page.locator("#right form.overview-edit");
  await expect(form).toBeVisible();
  await expect(form.locator('[name="name"]')).toHaveValue("kk-engine");
  await form.locator('[name="name"]').fill("kk-engine two");
  await form.locator('[name="description"]').fill("The engine, second edition");
  await form.locator('[name="about"]').fill("## Why\n\nFor **games**.");
  await form.locator('[data-action="overview-preview"]').click();
  await expect(form.locator(".ov-preview strong")).toHaveText("games");
  await form.locator('[name="links"]').fill("Repo https://git.kreative-kompas.com/khyretos/kk-engine\nDocs https://docs.example.com/kk");
  await form.locator('button[type="submit"]').click();
  await expect(form).toHaveCount(0);
  await expect(overview(page).locator(".ov-name")).toHaveText("kk-engine two");
  await expect(overview(page).locator(".ov-desc")).toHaveText("The engine, second edition");
  await expect(overview(page).locator(".ov-about strong")).toHaveText("games");
  await expect(sec(page, "where").locator('a.chip[href="https://docs.example.com/kk"]')).toHaveText("Docs");
  await expect(head(page, "p-kk").locator(".project-name")).toHaveText("kk-engine two");
  // The description is not only a hover tooltip any more: it is on the page.
  await expect(overview(page).locator(".ov-desc")).toBeVisible();
  expect(loads).toBe(0);
});

test("a refused edit puts the old values back and says why", async ({ page }) => {
  await openDemo(page);
  await head(page, "p-kk").click();
  await overview(page).locator('[data-action="overview-edit"]').click();
  const form = page.locator("#right form.overview-edit");
  await form.locator('[name="name"]').fill("Renamed");
  await form.locator('[name="links"]').fill("Files ftp://files.example.com");
  await form.locator('button[type="submit"]').click();
  await expect(page.locator(".toast").last()).toContainText("http or https");
  await expect(overview(page).locator(".ov-name")).toHaveText("kk-engine");
  await expect(head(page, "p-kk").locator(".project-name")).toHaveText("kk-engine");
});

test("an empty about invites the owner to add one", async ({ page }) => {
  await openDemo(page);
  await head(page, "p-kompanion").click();
  const about = overview(page).locator(".ov-about");
  await expect(about).toHaveClass(/empty/);
  await expect(about).toContainText("Add what this project is for");
  await about.locator('[data-action="overview-edit"]').click();
  await expect(page.locator("#right form.overview-edit [name=about]")).toBeFocused();
});

for (const scheme of ["light", "dark"] as const) {
  test(`phone: the project opens the Tasks pane on its overview, tiles 2x2, no sideways scroll (${scheme})`, async ({ page }) => {
    await openDemo(page, 1280, 900, scheme);
    await setUpKk(page);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.locator('[data-action="pane"][data-pane="left"]').first().click();
    await head(page, "p-kk").click();
    await expect(page.locator(".shell")).toHaveAttribute("data-pane", "right");
    const o = overview(page);
    await expect(o).toBeVisible();
    const boxes = await o.locator(".ov-tile").evaluateAll((els) => els.map((e) => e.getBoundingClientRect()).map((r) => ({ x: Math.round(r.x), y: Math.round(r.y) })));
    expect(boxes).toHaveLength(4);
    expect(boxes[0].y).toBe(boxes[1].y);
    expect(boxes[2].y).toBe(boxes[3].y);
    expect(boxes[2].y).toBeGreaterThan(boxes[0].y);
    expect(boxes[0].x).toBe(boxes[2].x);
    // Every section after the About is collapsed on a phone.
    for (const name of ["where", "access", "build", "activity", "ai", "settings"]) {
      await expect(secToggle(page, name)).toHaveAttribute("aria-expanded", "false");
    }
    await secToggle(page, "where").click();
    await expect(sec(page, "where").locator(".ov-folder")).toBeVisible();
    await expect(sec(page, "where").locator('[data-action="copy-folder"]')).toBeVisible();
    const wide = await page.evaluate(() => {
      const right = document.querySelector("#right") as HTMLElement;
      return { doc: document.documentElement.scrollWidth, right: right.scrollWidth - right.clientWidth };
    });
    expect(wide.doc).toBeLessThanOrEqual(390);
    expect(wide.right).toBeLessThanOrEqual(0);
    await page.screenshot({ path: `test-results/overview-390-${scheme}.png`, fullPage: true });
  });
}

test("the overview follows live changes without a reload", async ({ page }) => {
  await openDemo(page);
  let loads = 0;
  page.on("load", () => { loads++; });
  await setUpKk(page);
  // Changing the build command in Settings sends a "projects" change event; the overview reloads from it.
  await secToggle(page, "settings").click();
  const dev = page.locator("#right form.project-dev");
  await dev.locator('[name="build"]').fill("ninja -C out");
  await dev.locator('button[type="submit"]').click();
  await expect(sec(page, "build")).toContainText("ninja -C out");
  expect(loads).toBe(0);
});

test("tiles follow the width of the overview, not the window; the progress bar is plum", async ({ page }) => {
  await openDemo(page, 1280, 900, "dark");
  await head(page, "p-kk").click();
  const o = overview(page);
  await expect(o.locator(".ov-tiles .ov-tile")).toHaveCount(4);
  const columns = async () => (await o.locator(".ov-tiles").evaluate((el) => getComputedStyle(el).gridTemplateColumns)).split(" ").length;
  // A wide window with a narrow pane gives 2x2, a wide pane gives one row of four.
  await o.evaluate((el) => (el.style.width = "420px"));
  await expect.poll(columns).toBe(2);
  await o.evaluate((el) => (el.style.width = "760px"));
  await expect.poll(columns).toBe(4);
  // The progress bar uses the primary plum (--brand), not the orange accent.
  await expect(o.locator(".ov-bar > span")).toHaveCSS("background-color", "rgb(92, 57, 142)");
  const brand = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--brand").trim());
  expect(brand.toLowerCase()).toBe("#5c398e");
});
