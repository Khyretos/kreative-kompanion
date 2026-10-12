#!/bin/sh
set -eu

DIST_DIR="${DIST_DIR:-$(dirname "$0")/../dist}"
BIN="${BIN:-$HOME/.local/bin/kompanion-runner}"
RESTART_CMD="${RESTART_CMD:-systemctl --user restart kompanion-runner.service}"

# 1. If BIN is not an executable file, exit quietly.
[ -x "$BIN" ] || exit 0

# 2. Find the newest shipped build in DIST_DIR.
NEWEST=""
NEWEST_VER=""
for f in "$DIST_DIR"/kompanion-runner-*-x86_64-linux-musl; do
    [ -f "$f" ] || continue
    ver=$(basename "$f" | sed 's/kompanion-runner-\([0-9.]*\)-x86_64-linux-musl/\1/')
    if [ -z "$NEWEST_VER" ] || printf '%s\n%s\n' "$NEWEST_VER" "$ver" | sort -V | tail -n1 | grep -q "^${ver}$"; then
        NEWEST="$f"
        NEWEST_VER="$ver"
    fi
done
[ -n "$NEWEST" ] || exit 0

# 3. Get installed version from "$BIN --version".
INSTALLED_VER=$("$BIN" --version 2>/dev/null | awk '{print $2}') || INSTALLED_VER="0"

# 4. If installed >= newest, print message and exit.
if printf '%s\n%s\n' "$INSTALLED_VER" "$NEWEST_VER" | sort -V | tail -n1 | grep -q "^${INSTALLED_VER}$"; then
    echo "runner: $INSTALLED_VER is current"
    exit 0
fi

# 5. Update: copy to .new, chmod, mv, restart.
cp "$NEWEST" "${BIN}.new"
chmod 755 "${BIN}.new"
mv "${BIN}.new" "$BIN"
sh -c "$RESTART_CMD"
echo "runner: updated $INSTALLED_VER -> $NEWEST_VER"
