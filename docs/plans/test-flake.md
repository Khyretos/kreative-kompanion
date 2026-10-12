# TEST-FLAKE: the tasktree phone screenshot test (2026-10-12)

Before: web-live failed on PRs 189, 190 and 191 (also Rust-only ones) in `tasktree.spec.ts` "the tree fits a 390 px phone in light/dark mode" with "locator.screenshot: Element is not attached to the DOM": the demo app re-renders the tree between the visibility check and the screenshot.
After: the screenshot is taken inside `expect(async () => { await page.locator(".task-tree").screenshot(...) }).toPass({ timeout: 5000 })`, which looks the element up again on every try. Check: tsc clean and the two phone tests passed 8 repetitions each.

Coder run log: tf-screenshot (soucouyant), 4 attempts, failed (invented `toHaveScreenshot`); Claude pasted the given block. Lesson 195, situation docs/qwen-situations/195-ignores-the-given-line/.
