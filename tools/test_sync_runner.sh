#!/bin/sh
# RUNNER-SYNC: tests for tools/sync_runner.sh (fake dist dir, fake binary, fake restart command).
set -u
cd "$(dirname "$0")/.." || exit 1
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
fail() {
    echo "FAIL: $1"; exit 1
}
mkbin() {
    printf '#!/bin/sh\necho kompanion-runner %s\n' "$2" >"$1"
    chmod 755 "$1"
}
mkdir -p "$T/dist"
mkbin "$T/dist/kompanion-runner-0.5.0-x86_64-linux-musl" 0.5.0
mkbin "$T/dist/kompanion-runner-0.10.0-x86_64-linux-musl" 0.10.0
mkbin "$T/dist/kompanion-runner-0.9.1-x86_64-linux-musl" 0.9.1
echo "stub" >"$T/dist/kompanion-runner-0.10.0-x86_64-linux-musl.sha256"
run() { DIST_DIR="$T/dist" BIN="$T/kompanion-runner" RESTART_CMD="echo restarted >> $T/restarts" sh tools/sync_runner.sh; }

# 1. older runner: replaced by the numerically newest (0.10.0, not 0.9.1), restarted once
mkbin "$T/kompanion-runner" 0.5.0
run || fail "exit status with an older runner"
[ "$("$T/kompanion-runner" --version)" = "kompanion-runner 0.10.0" ] || fail "the runner was not replaced by 0.10.0"
[ "$(wc -l <"$T/restarts")" = 1 ] || fail "expected exactly one restart"
[ ! -e "$T/kompanion-runner.new" ] || fail "left a .new file"

# 2. already newest: nothing changes, no restart
run || fail "exit status when up to date"
[ "$(wc -l <"$T/restarts")" = 1 ] || fail "restarted although up to date"

# 3. runner newer than dist (a local build): left alone
mkbin "$T/kompanion-runner" 0.11.0
run || fail "exit status with a newer runner"
[ "$("$T/kompanion-runner" --version)" = "kompanion-runner 0.11.0" ] || fail "a newer runner was downgraded"
[ "$(wc -l <"$T/restarts")" = 1 ] || fail "restarted for a newer runner"

# 4. no runner installed, or an empty dist: skipped quietly with status 0
rm "$T/kompanion-runner"
run || fail "exit status without an installed runner"
[ ! -e "$T/kompanion-runner" ] || fail "installed a runner that was not there"
mkbin "$T/kompanion-runner" 0.5.0
rm "$T"/dist/kompanion-runner-*-musl
run || fail "exit status with an empty dist"
[ "$("$T/kompanion-runner" --version)" = "kompanion-runner 0.5.0" ] || fail "changed the runner with an empty dist"
echo "ok"
