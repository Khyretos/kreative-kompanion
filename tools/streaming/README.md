# Game streaming on soucouyant (Sunshine + Artemis)

See `docs/plans/apps3-streaming.md` for the choice and what was set up.

```sh
install -m 755 tools/streaming/kk-stream-display.sh ~/.local/bin/
cp tools/streaming/sunshine.conf tools/streaming/apps.json ~/.config/sunshine/
sudo sh tools/streaming/firewall.sh   # stream ports from LAN and WireGuard only
pkill -x sunshine; hyprctl eval 'hl.exec_cmd("sunshine")'
```

Pair a phone: in Artemis tap the PC, then type the PIN at <https://localhost:47990> (PIN tab)
on soucouyant.
