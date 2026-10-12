# Lesson 214: SCROLL-FIX was deployed from a checkout with a modified tracked file

Situation: queue thread 7 deployed SCROLL-FIX (PR 198, main 17022c3) with `sh tools/deploy.sh` from the kireserver main checkout `~/Docker/Personal-projects/kreative-kompanion`, fast-forwarded to origin/main. `git status --porcelain` there showed a modified tracked file, `docs/qwen-log/kg-01-task-tree.jsonl`, plus untracked backups (`.env.bak-stu-c4`, `kompanion.toml.bak-stu-c4`, `st.json`).

What the edit is (checked 2026-10-12 by queue thread 8, read only): one appended pipeline log line from a KG-01 Coder run on 2026-10-11 (job `kg01-tree-view`, `check_passed: false`). It is a Coder log, not code, so the deployed image was built from the same sources as origin/main. That was luck, not a check.

Rule: build and deploy only from a tree whose tracked files equal origin/main. Before `sh tools/deploy.sh` run `git status --porcelain --untracked-files=no` and stop if it prints anything, or deploy from a fresh worktree at origin/main with the untracked env files (`.env`, `kompanion.toml`) linked in. Do not touch the old kg-01 line in the main checkout; it belongs to Kees's checkout.

Not a Coder correction, so there is no job prompt or wrong output to rerun.
