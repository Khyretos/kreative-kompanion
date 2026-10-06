#!/bin/sh
# Push ntfy's public https address into kompanion.toml's [push] servers section.
# Edits the file in place (bind mount) without replacing it.

set -eu

URL="${KOMPANION_NTFY_URL:-}"

if [ -z "$URL" ] || ! echo "$URL" | grep -q '^https://'; then
    echo "push-setup: set KOMPANION_NTFY_URL in .env to ntfy's public https address" >&2
    exit 1
fi

f="${1:-}"
if [ -z "$f" ]; then
    echo "push-setup: usage: push-setup.sh <path to kompanion.toml>" >&2
    exit 1
fi

if ! grep -q '^\[push\]$' "$f"; then
    printf '\n[push]\nservers = ["%s"]\n' "$URL" >> "$f"
    echo "push-setup: added [push] servers"
    exit 0
fi

awk -v url="$URL" '
BEGIN { in_push = 0 }
/^\[push\]$/ { in_push = 1; print; next }
/^\[/ && in_push { in_push = 0; print; next }
in_push && /^servers *= *\[ *\] *$/ {
    print "servers = [\"" url "\"]"
    next
}
{ print }
' "$f" > /tmp/push-setup-$$

if cmp -s "$f" /tmp/push-setup-$$; then
    rm -f /tmp/push-setup-$$
    echo "push-setup: [push] servers already set, left as is"
    exit 0
fi

cat /tmp/push-setup-$$ > "$f"
rm -f /tmp/push-setup-$$
echo "push-setup: updated [push] servers"
