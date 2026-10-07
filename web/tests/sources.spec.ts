import { test, expect } from "@playwright/test";

// CHAT-04: an answer shows its sources: names as links in the text, the excerpts in an expandable list.
test("an answer lists its sources with excerpts, and a knowledge citation opens its source", async ({ page }) => {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.locator('#left [data-action="open-chat"]', { hasText: "Sourced answer" }).click();
  const body = page.locator(".msg .body").last();
  await expect(body.locator("a", { hasText: "Godot Engine" }).first()).toHaveAttribute("href", "https://godotengine.org/releases/4.5/");
  await expect(body).not.toContainText(":::sources");
  const list = body.locator("details.sources");
  await expect(list).not.toHaveAttribute("open", "");
  await expect(list.locator("summary")).toHaveText("Sources (2)");
  await body.locator("a", { hasText: "Godot Assistant / timer.md" }).first().click();
  await expect(list).toHaveAttribute("open", "");
  await expect(list.locator("li.hit")).toContainText("emits timeout when it reaches 0");
  await expect(list.locator("li").first().locator("blockquote")).toContainText("shader baking");
  await expect(list.locator("li").first().locator("a")).toHaveAttribute("target", "_blank");
});
