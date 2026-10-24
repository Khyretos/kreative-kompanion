import { test, expect } from "@playwright/test";

// TASK-OVERVIEW: inside the desktop app the web app reports its running tasks to the tray
// (a fake __TAURI__ here records the calls), once per change.
test("the desktop tray gets the running tasks, and again when one starts", async ({ page }) => {
  await page.addInitScript(() => {
    const calls: { cmd: string; args: unknown }[] = [];
    (window as unknown as { __kkTrayCalls: typeof calls }).__kkTrayCalls = calls;
    (window as unknown as { __TAURI__: unknown }).__TAURI__ = { core: { invoke: async (cmd: string, args: unknown) => { calls.push({ cmd, args }); } } };
  });
  await page.goto("/?demo");
  await page.click("button.found-server");
  const calls = () => page.evaluate(() => (window as unknown as { __kkTrayCalls: { cmd: string; args: { tasks: { id: string; title: string; step: string }[] } }[] }).__kkTrayCalls);
  await expect.poll(async () => (await calls()).filter((c) => c.cmd === "running_tasks").length).toBe(1);
  const first = (await calls()).find((c) => c.cmd === "running_tasks")!;
  expect(first.args.tasks).toEqual([{ id: "t1", title: "Add a --vk-validation flag and setting", step: "Building in the container" }]);
  await page.locator('#left [data-action="open-chat"]').first().click();
  await page.fill("#prompt", "Profile the shader cache");
  await page.press("#prompt", "Enter");
  await expect.poll(async () => (await calls()).filter((c) => c.cmd === "running_tasks").at(-1)?.args.tasks.length, { timeout: 15_000 }).toBe(2);
  // A redraw with the same tasks sends nothing new.
  const n = (await calls()).length;
  await page.click("#ov-chip");
  await page.click("#ov-chip");
  expect((await calls()).length).toBe(n);
});
