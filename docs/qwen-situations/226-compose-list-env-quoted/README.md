# Lesson 226: compose list-style env values with quotes (2026-10-12, admin-lock/NetBird thread)

- Job: edit wg-easy's compose file (job.json, unit "wg-compose"; input-wg.orig.yml).
- Coder (OVMS): 1 draft + 2 repair rounds, all failed on the same error:
  `WG_DEFAULT_DNS must be '${KK_WG_SERVER_IP}, 9.9.9.9'`. It wrote
  `- WG_DEFAULT_DNS='${KK_WG_SERVER_IP},9.9.9.9'` and `- WG_POST_UP="iptables ..."`.
  In list-style `environment:` (`- KEY=value`) quotes are part of the value, so the
  container would get `'10.8.0.1,9.9.9.9'` and a WG_POST_UP starting with `"`.
  It also skipped the requested DNAT comment.
- Claude corrected it (corrected-output.yml): no quotes, the comment added.
- Claude's own mistake in the same unit: the first check grepped `docker compose config`
  YAML, which folds long values over lines, and its -A/-D sed missed `-A INPUT`. Fixed by
  reading `config --format json`. The Coder's repairs were not affected (the DNS error
  came first every time).

Rule: in compose list-style environment entries never quote the value; quote only
in map style (`KEY: "value"`). Rerun: job.json against check.sh.

Related small fix (rd-compose-vars, remote-desktop compose): asked to add a comment line, the
Coder replaced an existing comment ("The PCs run Sunshine...") instead of adding one. Claude
restored it. Lesson: "add a comment" never removes an existing line.

Raw files (job.json, inputs, wrong and corrected outputs, check.sh) stay on kireserver in `~/Docker/docs/qwen-situations-pending/wg-compose-list-env-quotes/`: they hold network and firewall configuration, so they are kept out of this repository.
