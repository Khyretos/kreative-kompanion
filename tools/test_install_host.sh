#!/bin/sh
# HOST-01: checks tools/install-host.sh with stub docker and curl (no real install). Run: sh tools/test_install_host.sh
set -u
here=$(cd "$(dirname "$0")" && pwd)
S="$here/install-host.sh"
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
mkdir "$T/bin"
cat >"$T/bin/docker" <<'D'
#!/bin/sh
echo "$*" >> "$KK_T/docker.log"
[ -f "$KK_T/docker-fails" ] && { echo "boom" >&2; exit 1; }
echo "ABCD-1234"
D
cat >"$T/bin/curl" <<'D'
#!/bin/sh
echo "$*" >> "$KK_T/curl.log"
echo 'echo "installer got $1" >> "$KK_T/install.log"'
D
chmod +x "$T/bin/docker" "$T/bin/curl"
fail=0
ok() { if [ "$1" = "$2" ]; then echo "ok   $3"; else
    echo "error: tools/install-host.sh: $3: got [$1] want [$2]"
    fail=1
fi; }
run() {
    rm -f "$T"/*.log "$T/docker-fails"
    KK_T="$T" PATH="$T/bin:$PATH" KOMPANION_URL="" sh "$S" "$@" >"$T/out" 2>&1
    echo $?
}
log() { cat "$T/$1.log" 2>/dev/null; }

ok "$(run --mode service --runner no)" 0 "service, no runner: exit 0"
ok "$(log docker)" "" "service, no runner: no docker call"
ok "$(run --mode personal --runner yes --url https://k.example)" 0 "personal, runner: exit 0"
ok "$(log docker)" "exec kreative-kompanion kompanion-server pair-host" "the code comes from the container"
ok "$(log curl)" "-fsSL https://k.example/install.sh" "the runner installer comes from the server"
ok "$(log install)" "installer got ABCD-1234" "the installer gets the code"
ok "$(run --mode personal --runner yes --url https://k.example --container kk2)" 0 "another container name"
ok "$(log docker)" "exec kk2 kompanion-server pair-host" "--container is used"
ok "$(printf 's\n\n' | {
    rm -f "$T"/*.log
    KK_T="$T" PATH="$T/bin:$PATH" sh "$S" --url https://k.example >"$T/out" 2>&1
    echo $?
})" 0 "asked: service, Enter"
ok "$(log docker)" "" "service defaults to no"
ok "$(printf 'p\n\n' | {
    rm -f "$T"/*.log
    KK_T="$T" PATH="$T/bin:$PATH" sh "$S" --url https://k.example >"$T/out" 2>&1
    echo $?
})" 0 "asked: personal, Enter"
ok "$(log docker)" "exec kreative-kompanion kompanion-server pair-host" "personal defaults to yes"
ok "$(run --mode service --runner yes)" 2 "a runner needs the server's address"
ok "$(log docker)" "" "no code without an address"
touch "$T/docker-fails"
ok "$(
    KK_T="$T" PATH="$T/bin:$PATH" sh "$S" --mode service --runner yes --url https://k.example >"$T/out" 2>&1
    echo $?
)" 1 "no code: exit 1"
ok "$(log curl)" "" "no install without a code"
ok "$(run --mode other --runner no)" 2 "a wrong mode is refused"
exit $fail
