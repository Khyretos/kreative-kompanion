# Lesson 180: Coder copies a claim from the example file into a new file

Situation: job `gate-tools-sh` (GATE-TOOLS, 2026-10-12): write tools/gate-tools.sh modelled on tools/gate-server.sh (given as context).
Wrong output (3/3 runs identical, check passed first try): the comment said "this mirrors the `tools` job in build.yml". There is no such job in build.yml; the sentence is the model reading `gate-server.sh`'s "mirrors the `server` job in build.yml" and swapping the word.
Corrected: corrected-gate-tools.sh (one comment line). Rerun after model refinement: run job.json with pipeline.py (context = input-context-gate-server.sh) and check whether the comment now makes no claim that can be false.
Per-run records (every prompt and answer): runs/.
