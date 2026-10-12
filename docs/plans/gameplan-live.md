# GAMEPLAN-LIVE: the game planner survives a model that forgets the title (2026-10-12)

Before: the live check of M6-11 on kireserver (test user, project "OVR live check", the fox platformer description) failed after 3 rounds (2 min 10 s) with "The answer is not a game plan: missing field `title`": the orchestrator's answer had no top-level title and nothing in the error said what it had answered.
After: `title` and `goal` are optional in the model's answer and are filled from the user's description when empty (title: first sentence cut at 60 characters; goal: the whole description); a plan without any plan still fails with its reason and now ends with the text "Last answer:" and the first 300 characters of the last answer, so the next failure can be read.

Tests first (3 new, failing before): a plan without title or goal parses (lists stay required), titles come from the description (given ones stay, a long sentence is cut at 60), the last answer is in the error. `cargo test` 534 pass.

## Coder run log (new loop: one draft, up to 3 repairs, OVMS and soucouyant in parallel on different units)

| unit                                             | model      | attempts | result                                                                        |
| ------------------------------------------------ | ---------- | -------- | ----------------------------------------------------------------------------- |
| gp-fill-titles                                   | OVMS       | 4        | function right; added a duplicate `mod tests`; Claude removed it (lesson 193) |
| gp-handler-fill (call in `propose`)              | soucouyant | 1        | pass                                                                          |
| gp-title-defaults (serde default on title, goal) | OVMS       | 1        | pass                                                                          |
| gp-last-answer (answer snippet in the error)     | soucouyant | 1        | pass                                                                          |

Situation docs/qwen-situations/193-coder-adds-its-own-test-module/ (with every run). Live rerun after the deploy: result below.

## Live rerun

(added after the deploy)
