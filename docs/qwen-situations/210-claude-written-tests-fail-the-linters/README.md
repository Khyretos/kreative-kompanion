# Lesson 210: Claude's own test scripts failed MegaLinter (black, isort, ruff, shfmt, shellcheck)

Situation: RUNNER-SYNC PR 196, first CI run. `tools/e2e_runner_update.py` (a dense one-file script) failed black, isort and ruff (multiple imports on one line, unused import, a 177-character line); `tools/test_sync_runner.sh` failed shfmt (one-line functions, `< file`) and shellcheck SC2164 (`cd` without `|| exit`). Both were written by Claude, no Coder draft, and not linted before the push.
Fix: isort, black and shfmt ran in docker images on kireserver (mechanical), the import, the long line and the `cd` were edited by hand.
Rule: lint every new .py/.sh file before the first push with the same tools as CI: `docker run python:3.12 pip install black isort ruff` then `black`, `isort --profile black`, `ruff check`; `mvdan/shfmt:v3.12.0 -i 4 (an older shfmt keeps `a; b` on one line)`; `koalaman/shellcheck`. The Coder-drafted `tools/sync_runner.sh` passed all of them untouched.
