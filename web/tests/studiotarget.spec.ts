import { test, expect } from "@playwright/test";

// GPU-03: one "Studio runs on" control above the GPU cards; Off stops the studio everywhere.
test("Studio runs on: automatic, one GPU or off", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="capabilities"]');
  const box = page.locator("section.studio-target");
  await expect(box.locator("h2")).toHaveText("Studio runs on");
  const seg = box.locator('[role="group"][aria-label="Studio runs on"]');
  await expect(seg.locator("button")).toHaveText(["Automatic", "kireserver (a770)", "soucouyant (rx9070)", "Off"]);
  await expect(seg.locator('[data-target="auto"]')).toHaveAttribute("aria-pressed", "true");
  await expect(box.locator(".studio-target-cost")).toHaveText("The studio computer while its studio is on, else the fallback GPU");
  await expect(seg.locator('[data-target="a770"]')).toHaveAttribute("title", "Coder pauses while it runs");
  await seg.locator('[data-target="off"]').click();
  await expect(seg.locator('[data-target="off"]')).toHaveAttribute("aria-pressed", "true");
  await expect(seg.locator('[data-target="auto"]')).toHaveAttribute("aria-pressed", "false");
  await expect(box.locator(".studio-target-cost")).toHaveText("No studio jobs; the studio apps are stopped");
  const card = page.locator(".caps-group:has(#caps-gpus) li", { hasText: "rx9070" });
  await expect(card.locator(".gpu-mode-state")).toHaveText("Studio off: the studio apps are stopped and Ollama is unloaded. Studio jobs go elsewhere or wait.");
  await seg.locator('[data-target="rx9070"]').click();
  await expect(box.locator(".studio-target-cost")).toHaveText("Studio jobs run on soucouyant (rx9070).");
});
