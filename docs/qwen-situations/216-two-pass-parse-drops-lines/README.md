# Lesson 216: the Coder parsed in two passes and dropped lines that look like steps

Situation: TASK-FORM unit tf1, new file web/src/views/taskdesc.ts (composeDescription + parseDescription), OVMS. Job: tf1.json (first job `tf1`, then the repair job `tf1r`).

- Attempt 1 passed Claude's first check but review found four bugs the check did not cover: parseDescription removed every line that looks like a numbered step from the notes, also outside the Steps block ("Do this:\n1. a\n2. b" lost its list), a blank goal still wrote "**Goal:** ", spaces around a newline were not collapsed, and "**Goal:**G" (no space) was missed. wrong-output-attempt1-taskdesc.ts.gz.
- Claude added those cases to the check (tf1-test.mjs) and sent a repair job (tf1r) naming each bug and the fix: walk the lines once, consume the step lines right after the "**Steps**" header. The Coder passed on its 2nd repair attempt, but kept its two-pass structure with an `inNotes` flag: a numbered list at the very start ("1. a\n2. b") was still dropped and the last Goal line won instead of the first. wrong-output-repair-taskdesc.ts.gz.
- 3 attempts in total, so Claude replaced parseDescription with a single-pass loop (the compose half of the Coder's draft is kept) and added those two cases to the check.

Rules: (1) a check for a parser needs the cases where text must be kept, not only the happy shape; Claude's first check was too weak (lesson 187 family). (2) When the prompt asks for a single pass over the lines, the Coder still prefers "find the indexes, then filter everything else" and loses data; give the loop skeleton (for with i, consume with i++) in the prompt.
