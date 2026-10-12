# Lesson 228: netwatch.py CLI (2026-10-12, admin-lock/NetBird thread)

- Job: job.json (a ~400-line CLI in one unit; too big for one 9B job, the spec should have been
  split into render / check / mail units).
- Coder (OVMS): draft 272 s, then two repair rounds of 7-8 s each that changed 1 line and left
  the same SyntaxError: an f-string SQL with nested `"` / `'` quotes (line 263).
- Other defects in the draft, any of which would have done damage in production:
  1. render read network.env as the stack's old .env and wrote to a relative folder path;
  2. on an IP change it overwrote network.env with only a generated block (all other values lost);
  3. `--dry-run` still ran render for real;
  4. mail settings were read from os.environ, never from ~/.config/kk-cpuwatch/env;
  5. NETWATCH_FAKE_IPS defaulted to [""], so it never asked the real IP services;
  6. `urlopen(..., headers=...)` is not a valid call.
- Rules: (a) split a CLI into units of one command each; (b) SQL with both quote
  kinds goes in a plain (non-f) string built with .format or concatenation, or in triple quotes;
  (c) a rewrite of a config file must start from its current text (read, change one key, write);
  (d) dry-run must gate every side effect, including helpers it calls.

Raw files (job.json, inputs, wrong and corrected outputs, check.sh) stay on kireserver in `~/Docker/docs/qwen-situations-pending/netwatch-cli/`: they hold network and firewall configuration, so they are kept out of this repository.
