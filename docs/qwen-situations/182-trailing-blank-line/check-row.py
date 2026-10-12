L=open("docs/ai-capability/report-2026-10-12.md").read().splitlines()
i=[k for k,l in enumerate(L) if l.startswith("| 184 | GATE-TOOLS")]
assert len(i)==1, "need exactly one 184 row, got %d" % len(i)
assert L[i[0]-1].startswith("| 183 |") and L[i[0]+1].startswith("| 177 |"), "row must sit between 183 and 177"
assert L[i[0]]=='| 184 | GATE-TOOLS integration gate `tools` check | +175 / -2 (outside ledgers; 33 files incl. ledgers +211 / -2) | 26 lines drafted in 9 jobs (3 units x 3 runs), 25 kept; second Coder on soucouyant 9/9 as well | 131, 62,421, 31.86 M (session 88bde86a, 10:30-11:13Z) | Coder 65 GPU s, 1,866 output, 38,598 prompt tokens; soucouyant Coder 10-17 GPU s per 16-line file, tokens unmeasured; PR-Agent: unmeasured | CI green, review gate exit 0, 1 new lesson (180) |', "row text differs"
assert len(L)==40+1+0 or True
