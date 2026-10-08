import { test, expect, type Page } from "@playwright/test";

async function openChat(page: Page): Promise<void> {
  await page.goto("/?demo");
  await page.click("button.found-server");
  await expect(page.locator("#left")).toBeVisible();
  await page.click("#left .new-chat");
}

const png = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==", "base64");

test.describe("Attach files in chat (CHAT-08)", () => {
  test("pick, remove, send: chips in the composer and under the message", async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await openChat(page);
    await page.setInputFiles("#attach-input", [
      { name: "notes.md", mimeType: "text/markdown", buffer: Buffer.from("# Notes\nhello") },
      { name: "cat.png", mimeType: "image/png", buffer: png },
      { name: "extra.txt", mimeType: "text/plain", buffer: Buffer.from("x") },
    ]);
    const chips = page.locator("#attach-row .attach-chip");
    await expect(chips).toHaveCount(3);
    await expect(chips.nth(1).locator("img.attach-thumb")).toBeVisible();
    // Compact chips with 5px corners, not pills.
    expect(await chips.first().evaluate((el) => getComputedStyle(el).borderTopLeftRadius)).toBe("5px");
    await chips.nth(2).locator('[data-action="attach-remove"]').click();
    await expect(chips).toHaveCount(2);

    await page.fill("#prompt", "What do these say?");
    await page.press("#prompt", "Enter");
    const sent = page.locator("article.msg.user").last();
    await expect(sent.locator(".attach-sent .attach-chip")).toHaveCount(2);
    await expect(sent.locator(".attach-sent")).toContainText("notes.md");
    await expect(sent.locator(".body")).toHaveText("What do these say?");
    await expect(page.locator("#attach-row .attach-chip")).toHaveCount(0);
    expect(errors).toEqual([]);
  });

  test("a dropped file and a pasted picture are added; other types are refused", async ({ page }) => {
    await openChat(page);
    await page.evaluate(() => {
      const dt = new DataTransfer();
      dt.items.add(new File(["fn main() {}"], "main.rs", { type: "text/plain" }));
      const wrap = document.querySelector(".composer-wrap")!;
      wrap.dispatchEvent(new DragEvent("dragenter", { dataTransfer: dt, bubbles: true }));
      wrap.dispatchEvent(new DragEvent("drop", { dataTransfer: dt, bubbles: true, cancelable: true }));
    });
    await expect(page.locator("#attach-row .attach-chip")).toHaveCount(1);
    await page.evaluate((b64) => {
      const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
      const dt = new DataTransfer();
      dt.items.add(new File([bytes], "shot.png", { type: "image/png" }));
      document.querySelector("#prompt")!.dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true }));
    }, png.toString("base64"));
    await expect(page.locator("#attach-row .attach-chip")).toHaveCount(2);
    await page.setInputFiles("#attach-input", [{ name: "tool.exe", mimeType: "application/octet-stream", buffer: Buffer.from("MZ") }]);
    await expect(page.locator("#attach-row .attach-chip")).toHaveCount(2);
    await expect(page.locator("body")).toContainText("tool.exe");
  });

  test("files alone can be sent", async ({ page }) => {
    await openChat(page);
    await page.setInputFiles("#attach-input", [{ name: "a.txt", mimeType: "text/plain", buffer: Buffer.from("hi") }]);
    await page.press("#prompt", "Enter");
    await expect(page.locator("article.msg.user .attach-sent .attach-chip")).toHaveCount(1);
  });
});
