# Lesson 229: set -u with an optional env var (2026-10-13)

The Coder used `set -u` (as asked) and then `[ "$KK_RULES_ONLY" = "1" ]`; with the variable unset (the normal production start) the script would die on the unbound variable before starting NetBird. The check only ran with KK_RULES_ONLY=1, so it passed. Claude changed it to `${KK_RULES_ONLY:-}`.

Rule: under `set -u` every optional variable is read as `${VAR:-}`; and a check must cover the default path, not only the test switch.

Raw files (job.json, inputs, wrong and corrected outputs, check.sh) stay on kireserver in `~/Docker/docs/qwen-situations-pending/kk-exit-firewall-set-u/`: they hold network and firewall configuration, so they are kept out of this repository.
