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
    // IMG-VIEW: the picture shows as a preview that opens large; the document stays a chip.
    await expect(sent.locator(".attach-sent .attach-chip")).toHaveCount(1);
    await expect(sent.locator(".attach-sent .attach-img img")).toHaveCount(1);
    await sent.locator(".attach-sent .attach-img").click();
    await expect(page.locator("dialog.lightbox")).toBeVisible();
    await expect(page.locator("dialog.lightbox .lightbox-name")).toHaveText("cat.png");
    await page.keyboard.press("Escape");
    await expect(page.locator("dialog.lightbox")).toHaveCount(0);
    await expect(sent.locator(".attach-sent")).toContainText("notes.md");
    await expect(sent.locator(".body")).toHaveText("What do these say?");
    await expect(page.locator("#attach-row .attach-chip")).toHaveCount(0);
    expect(errors).toEqual([]);
  });

  test("a dropped file and a pasted picture are added; any other type too", async ({ page }) => {
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
    await expect(page.locator("#attach-row .attach-chip")).toHaveCount(3);
    await expect(page.locator("#attach-row")).toContainText("tool.exe");
  });

  test("files alone can be sent", async ({ page }) => {
    await openChat(page);
    await page.setInputFiles("#attach-input", [{ name: "a.txt", mimeType: "text/plain", buffer: Buffer.from("hi") }]);
    await page.press("#prompt", "Enter");
    await expect(page.locator("article.msg.user .attach-sent .attach-chip")).toHaveCount(1);
  });

  test("ATTACH-MENU: the paperclip opens a menu; Files or pictures opens the file picker", async ({ page }) => {
    await openChat(page);
    await page.click("#attach-btn");
    const menu = page.locator("#attach-menu");
    await expect(menu).toBeVisible();
    await expect(menu.locator('[data-attach="files"]')).toBeFocused();
    const chooser = page.waitForEvent("filechooser");
    await menu.locator('[data-attach="files"]').click();
    await (await chooser).setFiles([{ name: "a.txt", mimeType: "text/plain", buffer: Buffer.from("hi") }]);
    await expect(menu).toBeHidden();
    await expect(page.locator("#attach-row .attach-chip")).toHaveCount(1);
  });

  test("PASTE-LOG: a long pasted log becomes a text file, a short paste stays text", async ({ page }) => {
    await openChat(page);
    const paste = (text: string) => page.evaluate((t) => {
      const dt = new DataTransfer();
      dt.setData("text/plain", t);
      document.querySelector("#prompt")!.dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true }));
    }, text);
    const log = Array.from({ length: 60 }, (_, i) => `  File "/app/server.py", line ${i}, in do_GET\n    ~~~~~~^^^^`).join("\n");
    await paste(log);
    await expect(page.locator("#attach-row .attach-chip")).toContainText("pasted-text-1.txt");
    await expect(page.locator("#prompt")).toHaveValue("");
  });

  test("PASTE-LOG: a traceback typed in a message is not cut into code blocks", async ({ page }) => {
    await openChat(page);
    await page.fill("#prompt", 'File "/app/server.py", line 162\n    self._send_json(200, result)\n    ~~~~~~~~~~~~~~~^^^^^^\nBrokenPipeError: [Errno 32] Broken pipe');
    await page.press("#prompt", "Enter");
    const sent = page.locator("article.msg.user").last();
    await expect(sent.locator(".body")).toContainText("BrokenPipeError");
    await expect(sent.locator(".body pre")).toHaveCount(0);
  });
});
