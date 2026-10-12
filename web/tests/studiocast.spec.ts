// STU-N2: the Cast panel of the VN scene: two to four people, each with their own prompt, left to right.
import { test, expect, type Page } from "@playwright/test";

test.use({ viewport: { width: 390, height: 844 } });

async function open(page: Page) {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('#conv-head [data-action="pane"][data-pane="left"]');
  await page.click('[data-action="studio"]');
  return page.locator("#studio");
}

test("A cast scene takes a person per field and picks the workflow by their number", async ({ page }) => {
  const studio = await open(page);
  await expect(studio.locator(".studio-cast")).toHaveCount(0);
  // the 3 and 4 people workflows are no cards of their own
  await expect(studio.locator('.studio-types [role="radio"]', { hasText: "3 people" })).toHaveCount(0);
  await studio.locator('.studio-types [role="radio"]', { hasText: "VN scene, cast" }).click();
  const cast = studio.locator(".studio-cast");
  await expect(cast.locator("textarea")).toHaveCount(2);
  await expect(cast.locator('[data-action="studio-cast-remove"]')).toHaveCount(0);
  await studio.locator("#studio-prompt").fill("a school rooftop at sunset");
  // an empty person is named, nothing is made
  await studio.locator('.studio-form button[type="submit"]').click();
  await expect(studio.locator(".studio-form")).toContainText("Describe person 1.");
  await studio.locator("#studio-person-0").fill("1girl, long red hair, blue uniform");
  await studio.locator("#studio-person-1").fill("1boy, short black hair, white shirt");
  await cast.locator('[data-action="studio-cast-add"]').click();
  await expect(cast.locator("textarea")).toHaveCount(3);
  // typed text survives the re-render
  await expect(studio.locator("#studio-person-0")).toHaveValue("1girl, long red hair, blue uniform");
  await studio.locator("#studio-person-2").fill("1girl, short blonde hair, yellow dress");
  // the first person moves to the right
  await cast.locator('[data-action="studio-cast-move"][data-index="0"][data-dir="1"]').click();
  await expect(studio.locator("#studio-person-0")).toHaveValue("1boy, short black hair, white shirt");
  await expect(studio.locator("#studio-person-1")).toHaveValue("1girl, long red hair, blue uniform");
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await cast.scrollIntoViewIfNeeded();
    await page.screenshot({ path: `test-results/stu-n2-${scheme}.png`, fullPage: false });
  }
  const before = await studio.locator(".studio-runs > li").count();
  await studio.locator('.studio-form button[type="submit"]').click();
  await expect(studio.locator(".studio-runs > li")).toHaveCount(before + 1);
  await expect(studio.locator(".studio-runs > li").first()).toContainText("VN scene, 3 people");
});

test("People can be removed down to two and the panel never overflows the phone", async ({ page }) => {
  const studio = await open(page);
  await studio.locator('.studio-types [role="radio"]', { hasText: "VN scene, cast" }).click();
  const cast = studio.locator(".studio-cast");
  await cast.locator('[data-action="studio-cast-add"]').click();
  await cast.locator('[data-action="studio-cast-add"]').click();
  await expect(cast.locator("textarea")).toHaveCount(4);
  await expect(cast.locator('[data-action="studio-cast-add"]')).toHaveCount(0);
  await cast.locator('[data-action="studio-cast-remove"]').first().click();
  await cast.locator('[data-action="studio-cast-remove"]').first().click();
  await expect(cast.locator("textarea")).toHaveCount(2);
  await expect(cast.locator('[data-action="studio-cast-remove"]')).toHaveCount(0);
  const wide = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth);
  expect(wide).toBe(false);
});
