#!/bin/sh
# Sunshine ports on soucouyant: LAN and WireGuard only (replaces the open-to-all rules).
# Run once as root: sudo sh tools/streaming/firewall.sh
# Allowed sources: SUNSHINE_ALLOW_FROM (space-separated CIDRs), default the home LAN and
# wg-easy's client subnet on kireserver. Re-run after changing either network.
# TCP 47984 47989 48010, UDP 47998-48000 48002 48010. The web UI (47990) stays
# closed: sunshine.conf allows it from this PC only.
for p in 47984 47989 47990 47998 47999 48000 48010; do ufw delete allow "$p"; done
for p in 47998 47999 48000 48010; do ufw delete allow "$p/udp"; done
for src in ${SUNSHINE_ALLOW_FROM:-192.168.178.0/24 10.8.0.0/24}; do
    ufw allow from "$src" to any port 47984,47989,48010 proto tcp comment Sunshine
    ufw allow from "$src" to any port 47998:48000,48002,48010 proto udp comment Sunshine
done
ufw status | grep -E 'Sunshine|479|480'
