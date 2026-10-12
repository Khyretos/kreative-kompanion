# Lesson 190: two slips in one handler: a mixed-up type and a shadowed variable

Situation: job `gp-web-propose` (M6-11; job.json): fill in `propose` of server/src/gameplan_web.rs like `propose` in planner_web.rs.
Wrong output (soucouyant, 4 attempts, never compiled; wrong-gameplan_web.rs.txt): (1) it treated `gameplan::audio_infos()` (already `WfInfo`) like Studio workflows and read a field `workflow` that does not exist; (2) `let (name, description) = row...` shadowed the user's `description`, so the model got the project's description as "the game" and the proposal stored it as its reason, a silent bug that compiles.
Corrected: Claude (corrected-propose.rs.txt): `wf_infos` on the loaded workflows, then `extend(audio_infos())`; the project's description renamed `about`.
Lesson: shadowing a parameter name in a handler is invisible to compile errors; the prompt should name the variables (`description` = the game, `about` = the project's text). The unit's check could not catch it either; a handler test with a fake model answer would (planned with the live check).
