# Lesson 236: drafts that passed their check but were wrong (2026-10-13, PROJ-OVERVIEW o8 w17)

refresh_git (o8) passed its test (job done -> cache written) but polled forever when the computer never picks the job up: no deadline, the queued job never dropped (the prompt asked for both). activity() (w17) passed tsc and its needles but showed More only when `shown.length > 5`, which is never true; Playwright found it later.

Rule: a test per branch the prompt names (the timeout path, the "more than 5" boundary); grep needles prove presence, not logic.
