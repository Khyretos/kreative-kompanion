# Lesson 208: "directly above function X" became "inside function X, after its early return"

Situation: RUNNER-SYNC unit rsw7 (rsw7.json, OVMS, 3 attempts, failed): insert four top-level lines (a const and two exported functions) directly above `function machine(m: MachineStats): SafeHtml {`. The Coder inserted them, indented, inside the function body after its first `return` (invalid: `export` inside a function), once per attempt; the check message "must stand directly above function machine" did not change the position. The second edit (the button line) was right every time. wrong-output.diff.
Fix: Claude moved the four lines above the function.
Rule: for an insert "above line L", the SEARCH block must be L alone and the REPLACE block the new lines plus L; say "top level, no indentation". Candidate pipeline help: when the check output repeats, quote the line numbers where the text is and where it should be.
