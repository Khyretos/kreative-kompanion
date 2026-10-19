#!/bin/sh
# SHARED-LIVE unit checks: sourced, run from the repo root
export PATH="$HOME/.cargo/bin:$PATH" CARGO_TARGET_DIR="$HOME/Docker/Personal-projects/kreative-kompanion/server/target"
ctest() { # ctest <filter> <file>
    out=$(cd server && timeout 1500 nice cargo test -q --locked "$1" 2>&1)
    printf '%s\n' "$out" | awk '/^error/{p=1} /^warning/{p=0} p' | head -60 | sed "s|^|error: $2: |"
    printf '%s\n' "$out" | grep -E 'panicked|left:|right:|FAILED|test result' | head -30 | sed "s|^|error: $2: |"
    printf '%s\n' "$out" | grep -q 'test result: ok' || exit 1
    printf '%s\n' "$out" | grep -q 'FAILED' && exit 1
    return 0
}
need() { grep -qF -- "$2" "$1" || {
    echo "error: $1: missing: $2"
    exit 1
}; }
gone() { if grep -qF -- "$2" "$1"; then
    echo "error: $1: still has: $2"
    exit 1
fi; }
