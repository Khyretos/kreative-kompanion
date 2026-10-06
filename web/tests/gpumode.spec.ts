import { test, expect } from "@playwright/test";

// GPU-01: a computer with studio apps gets a Studio / Gaming / Auto switch on its GPU card.
test("soucouyant's GPU card switches between Auto, Gaming and Studio", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.click('[data-action="capabilities"]');
  const card = page.locator("#caps-gpus ~ .caps-cards li, .caps-group:has(#caps-gpus) li", { hasText: "rx9070" });
  const sw = card.locator('.gpu-mode[role="group"]');
  await expect(sw).toHaveAttribute("aria-label", "GPU mode on soucouyant");
  await expect(sw.locator("button")).toHaveText(["Studio", "Gaming", "Auto"]);
  await expect(sw.locator('[data-mode="auto"]')).toHaveAttribute("aria-pressed", "true");
  await expect(card.locator(".gpu-mode-state")).toHaveText("Auto: the studio apps stop after 15 min without a studio job.");
  await sw.locator('[data-mode="gaming"]').click();
  await expect(sw.locator('[data-mode="gaming"]')).toHaveAttribute("aria-pressed", "true");
  await expect(sw.locator('[data-mode="auto"]')).toHaveAttribute("aria-pressed", "false");
  await expect(card.locator(".gpu-mode-state")).toHaveText("Gaming: the studio apps are stopped and Ollama is unloaded.");
  await expect(card).toContainText("Nothing loaded");
  await sw.locator('[data-mode="studio"]').click();
  await expect(card.locator(".gpu-mode-state")).toHaveText("Studio apps stopped; the next studio job starts them on soucouyant.");
  // Kireserver's GPUs have no studio apps: no switch.
  await expect(page.locator(".caps-group:has(#caps-gpus) li", { hasText: "a770" }).locator(".gpu-mode")).toHaveCount(0);
});
