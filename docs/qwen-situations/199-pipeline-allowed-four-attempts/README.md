# Lesson 199: the pipeline allowed 4 attempts, the loop allows 3

Situation: GATE-IN-CI unit ru2 (lesson 197) ran 4 times (first run + 3 repair rounds) because `check_loop` in tools/qwen/pipeline.py used `range(1, 4)`; Kees's rule of 2026-10-12 14:24Z is 3 attempts in total. The coordinator spotted it in the metrics.
Fix: the Coder (OVMS, attempt 1, check passed) changed it to `range(1, 3)` and the docstring; a new test (tools/qwen/test_check_loop.py) fails with 4 attempts. Claude updated the module docstring line by script.
Rerun: job in jobs.json (prompt + check script ru9).
