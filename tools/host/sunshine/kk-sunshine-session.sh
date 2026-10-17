#!/bin/sh
# Hands the live session's variables to the user systemd manager, then (re)starts Sunshine so it always runs with the current session (SUN-01).

set -eu

dbus-update-activation-environment --systemd WAYLAND_DISPLAY DISPLAY HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE

systemctl --user restart app-dev.lizardbyte.app.Sunshine.service
