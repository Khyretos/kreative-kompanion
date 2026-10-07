#!/bin/sh
# HOST-01: run on the computer that hosts Kompanion, after `docker compose up -d` and the first account.
# Asks whether Kompanion may also act on THIS computer (a runner with no rights until you grant them in
# Access); if yes, pairs it with a code from the container, no copying from the web page.
# Usage: sh tools/install-host.sh [--mode personal|service] [--runner yes|no] [--url https://kompanion.example] [--container kreative-kompanion]

set -eu

MODE=""
RUNNER=""
URL="${KOMPANION_URL:-}"
C="${KOMPANION_CONTAINER:-kreative-kompanion}"

while [ $# -gt 0 ]; do
	case "$1" in
		--mode) MODE="$2"; shift 2 ;;
		--runner) RUNNER="$2"; shift 2 ;;
		--url) URL="$2"; shift 2 ;;
		--container) C="$2"; shift 2 ;;
		*) printf 'unknown option: %s\n' "$1" >&2; exit 2 ;;
	esac
done

ask() {
	printf '%s ' "$1" >&2
	read -r a || a=""
	echo "$a"
}

[ -n "$MODE" ] || MODE=$(ask "Is Kompanion your personal app on this PC (p) or a service for several people and computers (s)? [p/s]")
case "$MODE" in
	p|personal) MODE=personal ;;
	s|service) MODE=service ;;
	*) printf 'Answer p or s.\n' >&2; exit 2 ;;
esac

if [ -z "$RUNNER" ]; then
	# A personal install offers control of this PC by default; a service does not.
	if [ "$MODE" = personal ]; then def=y; q="[Y/n]"; else def=n; q="[y/N]"; fi
	RUNNER=$(ask "Also let Kompanion act on this computer? (a runner, no rights until you grant them in Access) $q")
	RUNNER=${RUNNER:-$def}
fi

case "$RUNNER" in
	y|Y|yes) ;;
	n|N|no) printf 'Nothing installed on this computer. You can pair it later in Machines > Pair.\n' >&2; exit 0 ;;
	*) printf 'Answer y or n.\n' >&2; exit 2 ;;
esac

if [ -z "$URL" ]; then
	echo "Set the server's address with --url (the address the runner connects to, e.g. https://kompanion.example)." >&2
	exit 2
fi

CODE=$(docker exec "$C" kompanion-server pair-host) || { echo "Could not get a pairing code from $C (see above)." >&2; exit 1; }
curl -fsSL "$URL/install.sh" | sh -s -- "$CODE"
