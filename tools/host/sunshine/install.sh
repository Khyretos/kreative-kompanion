#!/bin/sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
u=app-dev.lizardbyte.app.Sunshine.service
ud=$HOME/.config/systemd/user

# Step 1: Handle old full override
if [ -f "$ud/$u" ]; then
    if [ -f "$ud/$u.bak-sun01" ]; then
        echo "Error: $ud/$u.bak-sun01 already exists" >&2
        exit 1
    fi
    mv "$ud/$u" "$ud/$u.bak-sun01"
fi

# Step 2: Create directories
mkdir -p "$ud/$u.d" "$HOME/.local/bin"

# Step 3: Copy unit override
cp "$here/sun-01.conf" "$ud/$u.d/sun-01.conf"

# Step 4: Copy and make executable the session script
cp "$here/kk-sunshine-session.sh" "$HOME/.local/bin/kk-sunshine-session.sh"
chmod 755 "$HOME/.local/bin/kk-sunshine-session.sh"

# Step 5: Reload systemd user daemon
systemctl --user daemon-reload

# Step 6: Print Hyprland start-hook line
echo "hl.exec_cmd(\"sleep 3 && $HOME/.local/bin/kk-sunshine-session.sh\")"
