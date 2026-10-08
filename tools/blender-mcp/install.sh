#!/bin/sh
# BLD-01: Install the Blender MCP server as a systemd USER service.
# Usage: sh tools/blender-mcp/install.sh [--bind HOST:PORT] [--blender "CMD"] [--on-demand] [--no-check]
# This is opt-in: chats that use it can run any Python inside Blender as this user.

set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
D="$HOME/.local/share/kompanion/blender-mcp"
ENVF="$HOME/.config/kompanion/blender-mcp.env"
UNIT="$HOME/.config/systemd/user/kompanion-blender-mcp.service"
SOCKET="$HOME/.config/systemd/user/kompanion-blender-mcp.socket"

BIND="127.0.0.1:9876"
BLENDER=""
ONDEMAND=0
NOCHECK=0

while [ $# -gt 0 ]; do
    case $1 in
        --bind) BIND="$2"; shift 2 ;;
        --blender) BLENDER="$2"; shift 2 ;;
        --on-demand) ONDEMAND=1; shift ;;
        --no-check) NOCHECK=1; shift ;;
        *) echo "unknown option $1" >&2; exit 1 ;;
    esac
done

# server.py, scene.py, errors.py and vision.py use only the Python standard library: nothing to pip install.
command -v python3 >/dev/null 2>&1 || { echo "python3 is required" >&2; exit 1; }
python3 -c 'import sys; sys.exit(0 if sys.version_info >= (3, 8) else 1)' || { echo "python3 3.8 or newer is required" >&2; exit 1; }

# Always the latest Blender: the Flathub build is installed, or updated when it is already there.
if [ -z "$BLENDER" ]; then
    if command -v flatpak >/dev/null 2>&1; then
        if flatpak info org.blender.Blender >/dev/null 2>&1; then
            flatpak update -y --noninteractive org.blender.Blender || echo "warning: could not update Blender, using the installed one" >&2
        else
            flatpak install -y --noninteractive flathub org.blender.Blender || { echo "error: could not install Blender from Flathub" >&2; exit 1; }
        fi
        BLENDER="flatpak run --filesystem=/tmp org.blender.Blender"
    elif command -v blender >/dev/null 2>&1; then
        BLENDER="blender"
        echo "Using $(blender --version 2>/dev/null | head -1). Update it with your package manager; with flatpak installed this script would use the newest Flathub build."
    else
        echo "Blender not found: install flatpak (this script then installs the newest Blender) or pass --blender" >&2
        exit 1
    fi
fi

mkdir -p "$D"
cp "$HERE/server.py" "$HERE/scene.py" "$HERE/errors.py" "$HERE/vision.py" "$D/"

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
if [ "$ONDEMAND" = "1" ]; then
    cat > "$UNIT" <<EOF
[Unit]
Description=Kompanion Blender MCP (BLD-01)
[Service]
EnvironmentFile=$ENVF
ExecStart=/usr/bin/env python3 $D/server.py
Restart=on-failure
Environment=BLENDER_IDLE_EXIT=300
EOF

    mkdir -p "$(dirname "$SOCKET")"
    cat > "$SOCKET" <<EOF
[Unit]
Description=Kompanion Blender MCP socket
[Socket]
ListenStream=$BIND
[Install]
WantedBy=sockets.target
EOF
else
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
fi

systemctl --user daemon-reload
if [ "$ONDEMAND" = "1" ]; then
    systemctl --user enable --now kompanion-blender-mcp.socket
else
    systemctl --user enable --now kompanion-blender-mcp.service
fi

HOST="${BIND%:*}"; PORT="${BIND##*:}"
case "$HOST" in
    127.*|localhost) ;;
    *)
        # A LAN bind needs the firewall open for the local network (BUG-04: ufw dropped port 9876).
        SUBNET="${HOST%.*}.0/24"
        if [ "$(systemctl is-active ufw 2>/dev/null)" = active ]; then
            sudo -n ufw allow proto tcp from "$SUBNET" to any port "$PORT" >/dev/null 2>&1 \
                || echo "WARNING: ufw is active and blocks port $PORT. Run: sudo ufw allow proto tcp from $SUBNET to any port $PORT" >&2
        fi
        if [ "$(systemctl is-active firewalld 2>/dev/null)" = active ]; then
            { sudo -n firewall-cmd --permanent --add-port="$PORT/tcp" && sudo -n firewall-cmd --reload; } >/dev/null 2>&1 \
                || echo "WARNING: firewalld is active and blocks port $PORT. Run: sudo firewall-cmd --permanent --add-port=$PORT/tcp && sudo firewall-cmd --reload" >&2
        fi
        ;;
esac

if [ "$NOCHECK" = 0 ]; then
    if python3 -c "import socket,sys; socket.create_connection((sys.argv[1], int(sys.argv[2])), 5).close()" "$HOST" "$PORT" 2>/dev/null; then
        echo "ok: $BIND answers on this computer. From another computer check: curl -m 5 http://$BIND/mcp (405 is fine)."
    else
        echo "WARNING: $BIND does not answer on this computer; see: systemctl --user status kompanion-blender-mcp.socket" >&2
    fi
fi

echo "Installed. Add this to kompanion.toml and put the token in Kompanion's .env:"
echo "[[mcp]]"
echo "name = \"Blender\""
echo "url = \"http://$BIND/mcp\""
echo "token_env = \"MCP_BLENDER_TOKEN\""
echo "description = \"Headless Blender: builds and renders scenes from a chat\""
echo ""
echo "The token is in $ENVF (BLENDER_MCP_TOKEN). Use a LAN address in --bind if Kompanion runs on another computer."
