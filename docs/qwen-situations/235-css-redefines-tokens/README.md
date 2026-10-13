# Lesson 235: CSS job redefined the palette (2026-10-13, PROJ-OVERVIEW w29)

Asked to add Overview styles using the app's custom properties, the Coder added a new `:root { --surface: #f4f4f5; --fg: ...; --cta: #ea580c; ... }` block (it would override the Kompas palette and dark mode for the whole app) and unrelated `.archived-toggle[aria-pressed]` rules. Playwright checked layout only, so the check passed after my flaky-test fix. Claude removed both blocks in review.

Rule: a CSS job says "never define custom properties or :root rules; only selectors starting with .ov-", and the check fails on any new `--name:` definition or a selector outside the job's prefix.
