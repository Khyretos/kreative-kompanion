# Lesson 215: units with a new file were skipped as a spec error before the Coder ran

Situation: TASK-FORM units tf1 (new web/src/views/taskdesc.ts) and tf2 (new web/src/views/deppicker.ts). Claude wrote the job file without `"precheck": false`. pipeline.py runs every check once before any job; both checks failed with "file not written" (correctly, the files did not exist yet), so both jobs were skipped as spec errors and the Coder drafted nothing (log line `spec_error`).

Fix: every TASK-FORM job got `"precheck": false`, because each check was already proven by hand to fail on the untouched tree and to pass on a reference edit.

Rule: when a check is meant to fail before the job (new file, new export, test-first), set `"precheck": false` in the job. Not a Coder correction, so there is no wrong output to rerun.
