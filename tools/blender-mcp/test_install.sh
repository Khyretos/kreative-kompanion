#!/bin/sh
# BLD-01: install.sh in a fake HOME with a fake systemctl. Run: sh tools/blender-mcp/test_install.sh
F=tools/blender-mcp/install.sh
test -s $F || { echo "error: $F: not written"; exit 1; }
sh -n $F 2>&1 | sed "s|^|error: $F: |"
T=$(mktemp -d); mkdir -p $T/bin $T/home
printf '#!/bin/sh\necho "$@" >> %s/systemctl.log\n' "$T" > $T/bin/systemctl; chmod +x $T/bin/systemctl
fail=0
err() { echo "error: $F: $1"; fail=1; }
OUT=$(HOME=$T/home PATH=$T/bin:$PATH sh $F --bind 10.0.0.5:9999 --blender "flatpak run org.blender.Blender" 2>&1) || err "exit code $? ($OUT)"
D=$T/home/.local/share/kompanion/blender-mcp
test -f $D/server.py && test -f $D/scene.py || err "server.py and scene.py not copied to ~/.local/share/kompanion/blender-mcp"
E=$T/home/.config/kompanion/blender-mcp.env
test -f $E || err "no ~/.config/kompanion/blender-mcp.env"
[ "$(stat -c %a $E 2>/dev/null)" = 600 ] || err "env file mode is not 600"
grep -q '^BLENDER_MCP_BIND=10.0.0.5:9999$' $E || err "env: BLENDER_MCP_BIND=10.0.0.5:9999 missing"
grep -q '^BLENDER_CMD="flatpak run org.blender.Blender"$' $E || err 'env: BLENDER_CMD="flatpak run org.blender.Blender" missing'
TOK=$(sed -n 's/^BLENDER_MCP_TOKEN=//p' $E)
[ ${#TOK} -ge 32 ] || err "env: BLENDER_MCP_TOKEN shorter than 32 chars"
U=$T/home/.config/systemd/user/kompanion-blender-mcp.service
grep -q "^EnvironmentFile=$E$" $U 2>/dev/null || err "unit: EnvironmentFile=%h path missing"
grep -q "^ExecStart=/usr/bin/env python3 $D/server.py$" $U 2>/dev/null || err "unit: ExecStart line missing"
grep -q '^WantedBy=default.target$' $U 2>/dev/null || err "unit: WantedBy=default.target missing"
grep -q '^--user daemon-reload$' $T/systemctl.log 2>/dev/null || err "no systemctl --user daemon-reload"
grep -q '^--user enable --now kompanion-blender-mcp.service$' $T/systemctl.log 2>/dev/null || err "no systemctl --user enable --now"
echo "$OUT" | grep -q 'url = "http://10.0.0.5:9999/mcp"' || err "no [[mcp]] snippet with the url"
echo "$OUT" | grep -q "$TOK" && err "the token was printed"
# a second run keeps the token
HOME=$T/home PATH=$T/bin:$PATH sh $F --bind 10.0.0.5:9999 --blender "flatpak run org.blender.Blender" >/dev/null 2>&1
[ "$(sed -n 's/^BLENDER_MCP_TOKEN=//p' $E)" = "$TOK" ] || err "a second run changed the token"
rm -rf $T
[ $fail = 0 ] && echo "ALL PASS"
exit $fail
