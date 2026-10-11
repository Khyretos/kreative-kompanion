#!/bin/sh
# STU-C4b: install or update soucouyant's GPU guard as a user unit (run on soucouyant).
set -eu
here=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$HOME/.local/lib/gpu-guard" "$HOME/.config/systemd/user"
cp "$here/gpu_guard.py" "$HOME/.local/lib/gpu-guard/gpu_guard.py"
cp "$here/gpu-guard.service" "$HOME/.config/systemd/user/gpu-guard.service"
python3 -I "$HOME/.local/lib/gpu-guard/gpu_guard.py" --once
systemctl --user daemon-reload
systemctl --user enable --now gpu-guard.service
systemctl --user restart gpu-guard.service
systemctl --user --no-pager status gpu-guard.service | head -5
