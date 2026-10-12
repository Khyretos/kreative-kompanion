import { test, expect } from "@playwright/test";

test.describe("Desk notifications", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      (window as any).__notes = [];

      class FakeNotification {
        title: string;
        body: string;
        tag?: string;
        onclick: () => void;

        constructor(title: string, options?: { body?: string; tag?: string }) {
          this.title = title;
          this.body = options?.body ?? "";
          this.tag = options?.tag;
          this.onclick = () => {};
          (window as any).__notes.push({ title, body: this.body, tag: this.tag });
        }

        close() {
          // Implementation not needed for test
        }
      }

      (window as any).Notification = FakeNotification;
      (window as any).Notification.permission = "granted";
      (window as any).Notification.requestPermission = async () => "granted";

      (document as any).hasFocus = () => false;
    });

    page.on("dialog", (d) => d.accept()); // "Stop this task?"
    await page.goto("/?demo");
    await page.click("button.found-server");
  });

  test("off by default: no notification", async ({ page }) => {
    await page.click('[data-action="project"][data-id="p-kk"]');
    const taskLine = page.locator(".tasks .task-main", { hasText: "Add a --vk-validation flag and setting" });
    await taskLine.click();

    await page.click('[data-action="task-stop"]');
    await expect(page.locator("#right")).toContainText("stopped by you");

    const notes = await page.evaluate(() => (window as any).__notes ?? []);
    await expect(notes).toEqual([]);
  });

  test("switched on in Settings, a task that needs me raises one notification", async ({ page }) => {
    await page.click('[data-action="settings"]');
    await page.check("#desk-notify");
    await expect(page.locator("#desk-notify-msg")).toHaveText("On for this device.");
    await page.keyboard.press("Escape");

    await page.click('[data-action="project"][data-id="p-kk"]');
    const taskLine = page.locator(".tasks .task-main", { hasText: "Add a --vk-validation flag and setting" });
    await taskLine.click();

    await page.click('[data-action="task-stop"]');
    await expect(page.locator("#right")).toContainText("stopped by you");

    const count = await page.evaluate(() => ((window as any).__notes ?? []).length);
    await expect(count).toBe(1);

    const note = await page.evaluate(() => (window as any).__notes[0]);
    await expect(note.title).toBe("Kompanion: Add a --vk-validation flag and setting");
    await expect(note.body).toBe("Needs you");
  });
});

test.describe("Test notification button", () => {
  async function openSettings(page: import("@playwright/test").Page) {
    await page.goto("/?demo");
    await page.click("button.found-server");
    await page.click('[data-action="settings"]');
    await expect(page.locator("#desk-notify-test")).toBeVisible();
  }

  test("the button sends one notification even when the page has focus and the switch is off", async ({ page }) => {
    await page.addInitScript(() => {
      (window as any).__notes = [];
      class FakeNotification {
        constructor(title: string, options?: { body?: string }) {
          (window as any).__notes.push({ title, body: options?.body ?? "" });
        }
        close() {}
      }
      (window as any).Notification = FakeNotification;
      (window as any).Notification.permission = "granted";
      (window as any).Notification.requestPermission = async () => "granted";
      (document as any).hasFocus = () => true;
    });
    await openSettings(page);
    await expect(page.locator("#desk-notify")).not.toBeChecked();
    await page.click("#desk-notify-test");
    await expect(page.locator("#desk-notify-msg")).toContainText("Test notification sent");
    const notes = await page.evaluate(() => (window as any).__notes);
    expect(notes).toHaveLength(1);
    expect(notes[0].title).toBe("Kompanion test");
  });

  test("in the desktop app the test goes through the plugin and says so", async ({ page }) => {
    await page.addInitScript(() => {
      (window as any).__sent = [];
      (window as any).__TAURI__ = {
        notification: {
          isPermissionGranted: async () => true,
          requestPermission: async () => "granted",
          sendNotification: async (n: unknown) => { (window as any).__sent.push(n); },
        },
      };
    });
    await openSettings(page);
    await page.click("#desk-notify-test");
    await expect(page.locator("#desk-notify-msg")).toContainText("desktop app");
    const sent = await page.evaluate(() => (window as any).__sent);
    expect(sent).toHaveLength(1);
    expect(sent[0].title).toBe("Kompanion test");
  });

  test("a plugin failure is shown and logged, not swallowed", async ({ page }) => {
    const logged: string[] = [];
    page.on("console", (m) => { if (m.type() === "error") logged.push(m.text()); });
    await page.addInitScript(() => {
      (window as any).__TAURI__ = {
        notification: {
          isPermissionGranted: async () => true,
          requestPermission: async () => "granted",
          sendNotification: async () => { throw new Error("usernoted unreachable"); },
        },
      };
    });
    await openSettings(page);
    await page.click("#desk-notify-test");
    await expect(page.locator("#desk-notify-msg")).toContainText("usernoted unreachable");
    expect(logged.join("\n")).toContain("usernoted unreachable");
  });

  test("a denied permission is explained", async ({ page }) => {
    await page.addInitScript(() => {
      class FakeNotification { constructor() {} close() {} }
      (window as any).Notification = FakeNotification;
      (window as any).Notification.permission = "denied";
      (window as any).Notification.requestPermission = async () => "denied";
    });
    await openSettings(page);
    await page.click("#desk-notify-test");
    await expect(page.locator("#desk-notify-msg")).toContainText("blocked");
  });
});
