# GAMEPLAN-SOURCES: assets without a source get the matching workflow (2026-10-12)

Before: the live check of the game planner (fox platformer, test user) produced a good plan in 129 s (6 code tasks with correct dependencies, 3 asset tasks: sprite, sfx, music, each linked to the code tasks that need it) but no asset had a source, so the proposal carried 3 `no_source` issues.
After: `fill_sources` gives an asset task without a library asset or workflow the workflow named like its kind (sfx, music, sprite), else the first workflow with the output its kind needs; the asset's title becomes the `prompt` parameter when the workflow has one. `generate` runs it after the library swap and before the checks, so the plan the user approves has sources.

Tests first (4, failing before): kinds map to their workflows with the title as prompt; another image kind takes the first image workflow; nothing is filled when no workflow fits or a source exists; `generate` fills a missing source before the checks. `cargo test` 538 pass.

## Coder run log (one draft, up to 3 repairs, models in parallel)

| unit                              | model      | attempts | result                                                                                    |
| --------------------------------- | ---------- | -------- | ----------------------------------------------------------------------------------------- |
| gp-fill-sources                   | OVMS       | 2        | pass after one repair round (a `let ... else` that reassigned); 26 lines drafted, 26 kept |
| gp-call-fill (call in `generate`) | soucouyant | 1        | pass, 1 line kept                                                                         |

Situation docs/qwen-situations/194-retry-let-else-reassign/. No Claude correction, no new lesson.

## Live rerun

Result is added after the deploy.
