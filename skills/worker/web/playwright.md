---
extends: worker/web/playwright
---
20. A mock that hands its objects to the view must replace changed objects, never edit them in
    place: the view holds the same objects, so an in-place edit shows up without any event and a
    "live update" test passes for the wrong reason. `this.items = this.items.map((a) => changed.has(a.id) ? { ...a, x } : a)`.
25. (2026-10-04) No `test.skip` inside a test. "Unsaved edits ask first" checked for the e-mail field
    before it opened Settings, so it skipped on every run for a day and nobody noticed. If a test
    answers dialogs itself, call `page.removeAllListeners("dialog")` first: two handlers on one
    dialog throw "already handled".
34. (2026-10-05) Playwright's click() waits until the element stops moving; a message that
    re-renders 20-30 times a second (steps streaming) delays it by seconds under load, so the
    click lands in a later state (the step group had already closed itself). For state that
    changes by itself, read the state and click in one page.evaluate, then assert the opposite.
41. (2026-10-05) The demo needs `/?demo` plus a click on `button.found-server`; the dev server
    takes `PORT=<n>` (default 5173, often taken by another session).
61. (2026-10-06) Run the Playwright container without `--network host`: its own network keeps the
    webServer's port 5173 free even when a leftover dev server on the host holds it ("address
    already in use"), and the fix rounds don't start editing a test that never ran.
119. (2026-10-08) In the demo, `#left .new-chat` starts a loose chat (it clears the project); a chat in a project is `[data-action="new-chat"][data-project="<id>"]`. A button with `aria-disabled="true"` counts as disabled for Playwright: `click()` waits until the timeout, so check a locked control with `click({ force: true })` (OVR-01b).
