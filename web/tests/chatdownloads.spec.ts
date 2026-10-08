import { test, expect } from "@playwright/test";

// CHAT-05: pictures, files and code in a chat answer can be copied and downloaded.
const PNG = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==", "base64");

test("a Blender answer offers the picture and the .blend as downloads", async ({ page }) => {
  await page.route("**/api/chats/*/files/*.png", (r) => r.fulfill({ contentType: "image/png", body: PNG }));
  await page.route("**/api/chats/*/files/*.blend", (r) => r.fulfill({ contentType: "application/octet-stream", body: "BLENDER-v430" }));
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.locator('#left [data-action="open-chat"]', { hasText: "Blender scene" }).click();
  const body = page.locator(".msg .body").last();
  const fig = body.locator("figure.media");
  await expect(fig).toHaveCount(1);
  await expect(fig.locator("img")).toHaveCount(1);
  await expect(fig.locator('button[data-action="media-copy"]')).toHaveText("Copy");
  const png = page.waitForEvent("download");
  await fig.locator('button[data-action="media-download"]').click();
  expect((await png).suggestedFilename()).toBe("Blender_ blender_render.png");
  const card = body.locator(".media.file");
  await expect(card).toContainText("scene.blend");
  const blend = page.waitForEvent("download");
  await card.locator("a.btn").click();
  expect((await blend).suggestedFilename()).toBe("scene.blend");
});

test("a code block can be copied and downloaded", async ({ page }) => {
  await page.route("**/api/chats/*/files/*", (r) => r.fulfill({ contentType: "image/png", body: PNG }));
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.locator('#left [data-action="open-chat"]', { hasText: "Blender scene" }).click();
  const block = page.locator(".msg .body .codeblock").last();
  await expect(block.locator('button[data-action="code-copy"]')).toHaveText("Copy");
  const dl = page.waitForEvent("download");
  await block.locator('button[data-action="code-download"]').click();
  expect((await dl).suggestedFilename()).toBe("code.py");
});
