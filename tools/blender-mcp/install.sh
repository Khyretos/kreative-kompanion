#!/bin/sh
# BLD-01: Install the Blender MCP server as a systemd USER service.
# Usage: sh tools/blender-mcp/install.sh [--bind HOST:PORT] [--blender "CMD"]
# This is opt-in: chats that use it can run any Python inside Blender as this user.

set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
D="$HOME/.local/share/kompanion/blender-mcp"
ENVF="$HOME/.config/kompanion/blender-mcp.env"
UNIT="$HOME/.config/systemd/user/kompanion-blender-mcp.service"

BIND="127.0.0.1:9876"
BLENDER=""

while [ $# -gt 0 ]; do
    case $1 in
        --bind) BIND="$2"; shift 2 ;;
        --blender) BLENDER="$2"; shift 2 ;;
        *) echo "unknown option $1" >&2; exit 1 ;;
    esac
done

if [ -z "$BLENDER" ]; then
    if command -v blender >/dev/null 2>&1; then
        BLENDER="blender"
    elif flatpak info org.blender.Blender >/dev/null 2>&1; then
        BLENDER="flatpak run --filesystem=/tmp org.blender.Blender"
    else
        echo "Blender not found: install it or pass --blender" >&2
        exit 1
    fi
fi

mkdir -p "$D"
cp "$HERE/server.py" "$HERE/scene.py" "$D/"

mkdir -p "$(dirname "$ENVF")"
TOKEN=$(sed -n 's/^BLENDER_MCP_TOKEN=//p' "$ENVF" 2>/dev/null || true)
if [ -z "$TOKEN" ]; then
    TOKEN=$(python3 -c 'import secrets; print(secrets.token_hex(24))')
fi

(
    umask 077
    printf '%s\n%s\n%s\n' \
        "BLENDER_MCP_TOKEN=$TOKEN" \
        "BLENDER_MCP_BIND=$BIND" \
        "BLENDER_CMD=\"$BLENDER\"" \
    > "$ENVF"
)

mkdir -p "$(dirname "$UNIT")"
cat > "$UNIT" <<EOF
[Unit]
Description=Kompanion Blender MCP (BLD-01)
[Service]
EnvironmentFile=$ENVF
ExecStart=/usr/bin/env python3 $D/server.py
Restart=on-failure
[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable --now kompanion-blender-mcp.service

echo "Installed. Add this to kompanion.toml and put the token in Kompanion's .env:"
echo "[[mcp]]"
echo "name = \"Blender\""
echo "url = \"http://$BIND/mcp\""
echo "token_env = \"MCP_BLENDER_TOKEN\""
echo "description = \"Headless Blender: builds and renders scenes from a chat\""
echo ""
echo "The token is in $ENVF (BLENDER_MCP_TOKEN). Use a LAN address in --bind if Kompanion runs on another computer."
