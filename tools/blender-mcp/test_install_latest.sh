#!/bin/sh
# BUG-04: install.sh installs or updates Blender (latest), opens the firewall, and checks the port.
# Run: sh tools/blender-mcp/test_install_latest.sh
F=tools/blender-mcp/install.sh
T=$(mktemp -d)
mkdir -p $T/bin $T/home
fail=0
err() {
    echo "error: $F: $1"
    fail=1
}
fake() {
    printf '#!/bin/sh\n%s\n' "$2" >$T/bin/$1
    chmod +x $T/bin/$1
}
fake systemctl 'echo "$@" >> '$T'/systemctl.log; [ "$1 $2" = "is-active ufw" ] && [ -f '$T'/ufw-on ] && echo active; exit 0'
fake sudo 'echo "sudo $@" >> '$T'/sudo.log; exit 0'
fake ufw 'echo "ufw $@" >> '$T'/ufw.log; exit 0'

run() { HOME=$T/home PATH=$T/bin:/usr/bin:/bin sh $F --no-check --bind "$1" 2>&1; }
# 1. flatpak present, Blender not installed: installs it from flathub, uses it
fake flatpak 'echo "flatpak $@" >> '$T'/flatpak.log; [ "$1" = info ] && exit 1; exit 0'
OUT=$(run 192.168.178.80:9876) || err "exit $? ($OUT)"
grep -q 'flatpak install .*flathub org.blender.Blender' $T/flatpak.log || err "no flatpak install of org.blender.Blender from flathub"
grep -q '^BLENDER_CMD="flatpak run --filesystem=/tmp org.blender.Blender"$' $T/home/.config/kompanion/blender-mcp.env || err "BLENDER_CMD is not the flatpak run line"
# 2. already installed: it is updated, not reinstalled
: >$T/flatpak.log
fake flatpak 'echo "flatpak $@" >> '$T'/flatpak.log; exit 0'
OUT=$(run 192.168.178.80:9876) || err "exit $? ($OUT)"
grep -q 'flatpak update .*org.blender.Blender' $T/flatpak.log || err "no flatpak update of an installed Blender"
grep -q 'flatpak install' $T/flatpak.log && err "reinstalled an installed Blender"
# 3. ufw active and bind on a LAN address: opens the port; loopback does not touch the firewall
touch $T/ufw-on
OUT=$(run 192.168.178.80:9876) || err "exit $? ($OUT)"
grep -q 'ufw allow .*9876' $T/sudo.log $T/ufw.log 2>/dev/null || err "ufw active but the port was not allowed"
: >$T/sudo.log
: >$T/ufw.log
OUT=$(run 127.0.0.1:9876) || err "exit $? ($OUT)"
[ -s $T/sudo.log ] || [ -s $T/ufw.log ] && err "loopback bind touched the firewall"
# 4. sudo cannot run (password needed): the exact command is printed, exit 0
fake sudo 'exit 1'
OUT=$(run 192.168.178.80:9876) || err "exit $? when sudo failed ($OUT)"
echo "$OUT" | grep -q 'sudo ufw allow' || err "no printed ufw command when sudo failed"
rm -rf $T
[ $fail = 0 ] && echo "ALL PASS"
exit $fail
