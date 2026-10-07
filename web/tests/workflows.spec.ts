import { test, expect } from "@playwright/test";

// STU-01 (M6-05 step 4): the Capabilities page lists the saved workflows with licences, GPUs and runs.
test("Capabilities shows the workflows with licences, warnings, GPUs and run numbers", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="capabilities"]');
  const group = page.locator(".caps-group:has(#caps-workflows)");
  await expect(group.locator("h2")).toContainText("Workflows");
  const cards = group.locator("li");
  await expect(cards).toHaveCount(3);
  const base = cards.filter({ hasText: "Z-Image Turbo: text to image" });
  await expect(base.locator(".chip.state")).toHaveText("ready");
  await expect(base).toContainText("Apache-2.0 · on a770, rx9070");
  await expect(base).toContainText("12 runs · 23 s on average");
  const preset = cards.filter({ hasText: "Landscape" });
  await expect(preset).toContainText("Image type, based on z-image-turbo");
  await expect(preset).toContainText("No runs yet");
  const warned = cards.filter({ hasText: "Old SD 1.5 sprites" });
  await expect(warned.locator(".chip.state")).toHaveText("licence warning");
  await expect(warned.locator(".wf-problems")).toHaveText("licence warning: sd15.ckpt: CreativeML OpenRAIL-M: OpenRAIL licence, check its use restrictions");
});

// LIC-01: a type whose model licence is not OSI/permissive still runs; its card shows the warning.
test("Studio type cards show a licence warning", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="studio"]');
  // STU-UI1: the card shows a warning badge; the full text is in the detail panel.
  const card = page.locator('#studio .studio-type[data-type="oc-sheet"]');
  await expect(card.locator('.studio-badge.warn [role="img"]')).toHaveAttribute("aria-label", "Licence warning");
  await expect(page.locator('#studio .studio-type[data-type="character"] .studio-badge.warn')).toHaveCount(0);
  await card.click();
  await expect(page.locator("#studio-type-detail .studio-warning")).toHaveText("Licence warning: novaAnimeXL_ilV170.safetensors: Fair AI Public License 1.0-SD: unknown licence, check it before use");
});
