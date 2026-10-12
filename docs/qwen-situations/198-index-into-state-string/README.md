# Lesson 198: the Coder indexes a string result as if it were a tuple

Situation: GATE-IN-CI, job `ru4` (soucouyant, 1 attempt): `main()` prints "PR #n: <state>" for the dict that `sweep` returns ({number: state string}).
Wrong output: `print(f"PR #{n}: {results[n][0]}")`, which prints the first letter of the state ("s" for success). The check only called main with an empty env, so it passed (runs/wrong-souc-review_status_io.py.txt).
Corrected: Claude removed `[0]` (1 character).
Lesson: say what the return value looks like ("{int: str}") in the prompt, and give the check a run with a fake sweep that prints. Rerun after refinement: ru4 in jobs-all.json is replaced by jobs.json later; the prompt is in this folder's notes.
