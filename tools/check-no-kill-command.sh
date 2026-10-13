#!/bin/sh
# Fails when Rust code starts the `kill` program. procps 4.x reads `kill -KILL -1234567` as `kill -KILL -1` and signals every process of the user (2026-10-13, RUNNER-WIN). Use libc::kill with a guard instead.

set -eu
cd "$(dirname "$0")/.."

hits=$(grep -rn --include='*.rs' -E 'Command::new\("kill"\)' runner server gpu-helper machine-stats 2>/dev/null || true)

if [ -n "$hits" ]; then
    echo "no-kill-command: do not run the kill program, use libc::kill with a guard:" >&2
    printf '%s\n' "$hits" >&2
    exit 1
fi

echo "no-kill-command: ok"
