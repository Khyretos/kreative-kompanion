import { test, expect } from "@playwright/test";

// BLD-01: pictures from chat tools (Blender renders) show in the answer; other image links do not load.
const PNG = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==", "base64");

test("a Blender render shows in the chat, outside pictures do not", async ({ page }) => {
  const outside: string[] = [];
  await page.route("**/api/chats/*/files/*", (r) => r.fulfill({ contentType: "image/png", body: PNG }));
  page.on("request", (r) => { if (r.url().startsWith("https://example.com")) outside.push(r.url()); });
  await page.goto("/?demo");
  await page.click("button.found-server");
  await page.locator('#left [data-action="open-chat"]', { hasText: "Blender scene" }).click();
  const body = page.locator(".msg .body").last();
  const img = body.locator("img");
  await expect(img).toHaveCount(1);
  await expect(img).toHaveAttribute("src", "/api/chats/c-blender/files/0f8e2c1a-5b7d-4c3e-9a21-6d4f0b9e7c55.png");
  await expect(img).toHaveAttribute("alt", "Blender: blender_render");
  await expect(img).toHaveAttribute("loading", "lazy");
  await expect.poll(() => img.evaluate((el: HTMLImageElement) => el.complete && el.naturalWidth)).toBe(1);
  const css = await img.evaluate((el) => { const s = getComputedStyle(el); return { maxWidth: s.maxWidth, radius: s.borderTopLeftRadius, display: s.display }; });
  expect(css).toEqual({ maxWidth: "100%", radius: "8px", display: "block" });
  await expect(body).toContainText("Here is Suzanne");
  expect(outside).toEqual([]);
});
