# Lesson 195: the Coder ignores a replacement line that is given verbatim

Situation: TEST-FLAKE (2026-10-12), job `tf-screenshot` (jobs.json, soucouyant): in web/tests/tasktree.spec.ts replace one `tree.screenshot(...)` line by a given `expect(async () => { ... }).toPass({ timeout: 5000 })` block. The flake: the app re-renders the tree between the visibility check and the screenshot ("Element is not attached to the DOM"), failing web-live on every recent PR.
Wrong output (4 attempts): the Coder wrote `await expect(tree).toPass(async () => { await expect(tree).toHaveScreenshot(...) })`, a different API that needs saved baseline images, so all 16 repetitions of the check failed (wrong-tasktree.spec.ts.txt).
Corrected: Claude pasted the given block; tsc clean, the two phone tests passed 8 repetitions each. Same pattern as lesson 192 (a verbatim line given, not applied).
Lesson: for a replacement that is fully specified, apply it by script and let the Coder do only what needs judgement; or put the exact block alone in a "new text" section. Rerun after model refinement: run jobs.json.
