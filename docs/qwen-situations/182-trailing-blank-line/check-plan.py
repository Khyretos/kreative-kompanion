t = open("docs/plans/gate-tools.md").read()
tail = "## Metrics (step 10)\n\n| item | value |\n|---|---|\n| Claude, session 88bde86a, 10:30-11:13Z | 131 answers, 62,421 output, 278 input, 31,855,641 cache read, 219,101 cache write |\n| Coder (OVMS) | 9 jobs, 65 GPU s, 1,866 output tokens, 38,598 prompt tokens |\n| Coder (soucouyant Ollama) | 9 jobs, 10-17 GPU s for the 16-line file, tokens unmeasured |\n| Lines | 26 drafted, 25 kept |\n| PR-Agent | unmeasured |\n"
assert t.count("## Metrics (step 10)") == 1, "need exactly one Metrics heading"
assert t.endswith(
    tail
), "file must end exactly with the block, one final newline (no extra blank line)"
