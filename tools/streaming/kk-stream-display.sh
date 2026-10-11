#!/usr/bin/env bash
# Sunshine app helper for Hyprland: stream a headless virtual output sized to the
# client (the phone) instead of a real monitor. Real monitors are never touched.
#   do:   kk-stream-display.sh start   (Sunshine sets SUNSHINE_CLIENT_WIDTH/HEIGHT/FPS)
#   undo: kk-stream-display.sh stop
# sunshine.conf has output_name = KK-STREAM, so Sunshine captures this output while it
# exists and falls back to the first real monitor when it does not ("Desktop" app).
set -u
OUT="${KK_STREAM_OUTPUT:-KK-STREAM}"
STATE="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/kk-stream-display.prev"

# Sunshine runs outside the Hyprland session environment; find the live instance.
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
if [ -z "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]; then
    HYPRLAND_INSTANCE_SIGNATURE=$(find "$XDG_RUNTIME_DIR/hypr" -mindepth 1 -maxdepth 1 -printf '%T@ %f\n' 2>/dev/null | sort -rn | head -1 | cut -d" " -f2)
    export HYPRLAND_INSTANCE_SIGNATURE
fi

exists() { hyprctl monitors -j | jq -e --arg m "$OUT" 'any(.[]; .name==$m)' >/dev/null; }

case "${1:-}" in
start)
    w="${SUNSHINE_CLIENT_WIDTH:-2400}"
    h="${SUNSHINE_CLIENT_HEIGHT:-1080}"
    fps="${SUNSHINE_CLIENT_FPS:-60}"
    # Remember which monitor had focus, so stop can give it back
    hyprctl monitors -j | jq -r '.[]|select(.focused)|.name' >"$STATE"
    exists || hyprctl output create headless "$OUT" >/dev/null
    # Hyprland 0.56 with a Lua config: monitors are set with hyprctl eval (keyword is gone).
    # A mode sent right after create is lost, so apply it until the output reports it.
    for _ in 1 2 3 4 5 6 7 8 9 10; do
        hyprctl eval "hl.monitor({ output = \"$OUT\", disabled = false, mode = \"${w}x${h}@${fps}\", position = \"auto-right\", scale = 1.0 })" >/dev/null
        sleep 0.3
        hyprctl monitors -j | jq -e --arg m "$OUT" --argjson w "$w" --argjson h "$h" \
            'any(.[]; .name==$m and .width==$w and .height==$h)' >/dev/null && break
    done
    # New windows (Steam Big Picture, games) open on the focused monitor
    hyprctl eval "hl.dispatch(hl.dsp.focus({ monitor = \"$OUT\" }))" >/dev/null
    ;;
stop)
    exists && hyprctl output remove "$OUT" >/dev/null # Hyprland moves its windows to a real monitor
    prev=$(cat "$STATE" 2>/dev/null)
    [ -n "$prev" ] && [ "$prev" != "$OUT" ] && hyprctl eval "hl.dispatch(hl.dsp.focus({ monitor = \"$prev\" }))" >/dev/null
    rm -f "$STATE"
    ;;
*)
    echo "usage: $0 start|stop" >&2
    exit 2
    ;;
esac
exit 0
