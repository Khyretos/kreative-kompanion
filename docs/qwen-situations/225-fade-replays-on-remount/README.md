# Lesson 225: a CSS fade-in on rendered rows replays on every re-render

Situation: PROJ-ARCHIVE, found in the live check on 127.0.0.1:8095 (2026-10-13), not a Coder mistake. Claude's spec and the w8 job asked for `animation: kk-archive-in` on `.project.archived` and `.archived-bar`. The sidebar and the right pane are remounted on every store change, so the animation started again each time anything changed and archived rows flickered faint. At 390 px the Archived chip plus the done/total count also left the name about 50 px, so names read "Archi…".

Fix (unit p1, Coder first try, identical to the reference): drop the animation, hide the count on archived rows and let their name wrap. The Playwright test "archived rows do not fade in again on a re-render, and keep room for their name" checks `animationName` is `none`, the count is hidden and the name is not cut.

Rule: in this app (remount on change) a CSS entry animation on a rendered element replays on every render. Animate only on a class that is added once for the change itself, or not at all; and check a row's text at 390 px with all its chips present, not only the button size.
