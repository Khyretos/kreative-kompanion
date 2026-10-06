import { test, expect } from "@playwright/test";

// STU-01 (M6-05 step 4): the Capabilities page lists the saved workflows with licences, GPUs and runs.
test("Capabilities shows the workflows with licences, GPUs and run numbers", async ({ page }) => {
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
  const refused = cards.filter({ hasText: "Old SD 1.5 sprites" });
  await expect(refused.locator(".chip.state")).toHaveText("licence refused");
  await expect(refused.locator(".wf-problems")).toHaveText("sd15.ckpt: CreativeML OpenRAIL-M is not allowed");
});
