#!/bin/sh
# BLD-02: install.sh --on-demand writes a socket unit and no always-on service. Run: sh tools/blender-mcp/test_install_ondemand.sh
F=tools/blender-mcp/install.sh
T=$(mktemp -d); mkdir -p $T/bin $T/home
printf '#!/bin/sh\necho "$@" >> %s/systemctl.log\n' "$T" > $T/bin/systemctl; chmod +x $T/bin/systemctl
fail=0
err() { echo "error: $F: $1"; fail=1; }
HOME=$T/home PATH=$T/bin:$PATH sh $F --no-check --on-demand --bind 10.0.0.5:9999 --blender "blender" >/dev/null 2>&1 || err "exit code $?"
U=$T/home/.config/systemd/user
grep -q '^ListenStream=10.0.0.5:9999$' $U/kompanion-blender-mcp.socket 2>/dev/null || err "socket: ListenStream=10.0.0.5:9999 missing"
grep -q '^WantedBy=sockets.target$' $U/kompanion-blender-mcp.socket 2>/dev/null || err "socket: WantedBy=sockets.target missing"
grep -q '^Environment=BLENDER_IDLE_EXIT=300$' $U/kompanion-blender-mcp.service 2>/dev/null || err "service: Environment=BLENDER_IDLE_EXIT=300 missing"
grep -q '^WantedBy=' $U/kompanion-blender-mcp.service 2>/dev/null && err "service must not have an [Install] WantedBy"
grep -q '^--user enable --now kompanion-blender-mcp.socket$' $T/systemctl.log 2>/dev/null || err "no systemctl --user enable --now of the socket"
grep -q 'enable --now kompanion-blender-mcp.service' $T/systemctl.log 2>/dev/null && err "the service itself was enabled"
rm -rf $T
[ $fail = 0 ] && echo "ALL PASS"
exit $fail
