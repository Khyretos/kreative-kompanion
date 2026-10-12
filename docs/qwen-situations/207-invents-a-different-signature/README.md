# Lesson 207: the Coder wrote a different signature than the one given

Situation: RUNNER-SYNC unit rsw2 (rsw2.json): add `updateRunner(id: string): Promise<void>;` with a doc comment directly after the `unpairMachine` line of the KompanionApi interface (web/src/api/client.ts). The Coder (soucouyant, 3 attempts) added `/** W2: updates the runner script for a machine. */ updateRunner(runnerId: string, text: string): Promise<void>;` at a different place (after moveLesson); the check named the position error each round and it did not move. wrong-output.diff.gz.
Fix: Claude removed the two lines and added the intended ones.
Rule: soucouyant's Ollama model copied the method name but invented parameters; when the prompt gives the full line, say "copy this line character for character" and put the line in a fenced block, as the OVMS model handles prose better. Both failures this task (203, 207) were soucouyant on tiny insert jobs.
