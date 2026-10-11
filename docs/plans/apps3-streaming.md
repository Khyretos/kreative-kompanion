# Apps 3: game streaming and remote control, soucouyant → Kees's Android phone

Research for Kompanion task "Apps 3 (B)" (2026-10-05). Kees picked Sunshine + Artemis;
set up on soucouyant on 2026-10-11 (see "Installed" below), the phone is his hands.

## soucouyant in short

CachyOS, **Hyprland** (wlroots-family Wayland compositor), AMD RX 9070 XT (RDNA4: VAAPI
HEVC/AV1 encode), gaming and development desktop. Kees's rule: FOSS only.

## Hosts (on soucouyant)

| Host | Licence | Linux status | Virtual display on Linux | Fits Hyprland? | Notes |
|---|---|---|---|---|---|
| **Sunshine** (LizardByte) | GPL-3.0 | Mature, packaged for Arch/CachyOS | No built-in one; Hyprland can make one itself (`hyprctl output create headless`) | Yes: wlr-screencopy / KMS capture, VAAPI | The baseline, most users, best docs |
| Apollo (ClassicOldSong) | GPL-3.0 | Works, but its headline feature is Windows-only | **Not on Linux yet** (planned) | Partly | Pairs best with Artemis (auto resolution, per-client permissions) |
| Apollo-Linux (community fork) | GPL-3.0 | Small fork | EVDI kernel module | Unclear | Few users, kernel module to maintain |
| Vibepollo (Nonary) | not stated in README | **Beta**, targets Arch/CachyOS with **KDE Plasma 6** | Own kernel driver | Not mentioned (Plasma only) | Says ~99 % AI-generated; licence unclear: not for now |

## Clients (on the phone)

| Client | Licence | Notes |
|---|---|---|
| **Artemis** (Moonlight fork for Android) | GPL-3.0 | Custom resolutions and bitrates, several mouse modes (touchpad, multi-touch), virtual gamepad skins, external-monitor mode, lower MediaTek decode latency merged in; works with Sunshine, extra tricks with Apollo |
| Moonlight Android | GPL-3.0 | Upstream, stable, fewer features |

Android adds 1-3 frames of display latency on any client; the decoder fix in Artemis
matters most on MediaTek phones.

## Recommendation

**Sunshine on soucouyant + Artemis on the phone**, with a Hyprland headless output as the
virtual display (phone resolution, monitors untouched, removed when the stream ends).
Revisit Apollo when its Linux virtual display ships, and Vibepollo when it leaves beta,
supports wlroots/Hyprland and states a licence.

## Setup plan (after Kees picks)

1. soucouyant (soucouyant thread): install `sunshine` from the CachyOS/Arch repo; capture
   method wlr (Hyprland) with KMS as fallback; encoder VAAPI (HEVC, AV1 if the phone decodes
   it); firewall: Sunshine ports only from the LAN and WireGuard; web UI behind localhost.
2. Virtual display: a Sunshine "prep" command per app that runs
   `hyprctl output create headless KK-STREAM` and sets it to the phone's resolution and
   refresh, moves the game there, and removes it on stream end.
3. Apps in Sunshine: "Desktop" (remote control of the normal desktop) and Steam Big Picture.
4. Phone (Kees): Artemis APK from its GitHub releases, pair with the PIN, HEVC, bitrate
   20-40 Mbit/s on Wi-Fi, gamepad mapping.
5. Measure: on-screen timer photo for latency, dropped frames and bitrate at 1080p60 and
   at the phone's native resolution, over Wi-Fi and WireGuard; write the numbers here.

## Installed (2026-10-11)

Files in `tools/streaming/`, installed as `~/.local/bin/kk-stream-display.sh` and
`~/.config/sunshine/{sunshine.conf,apps.json}` (the old ones are kept as `*.bak-2026-10-11`).

- Sunshine 2026.1006 (CachyOS package) was already installed and starts from Hyprland's
  `hyprland.start` hook. Capture `wlr`, encoder `vaapi`: the startup probe finds
  `h264_vaapi`, `hevc_vaapi` and `av1_vaapi` on the RX 9070 XT.
- `output_name = KK-STREAM`: Sunshine (since #5071) matches the xdg_output name, so it streams
  the headless output while it exists. Without it the name does not parse as an index and
  Sunshine takes the first monitor (DP-1). Both cases checked in Sunshine's startup log.
- Apps: **Phone screen** (desktop on a headless output at the phone's resolution and fps),
  **Steam Big Picture (phone screen)**, and **Desktop (my monitor)** (mirrors DP-1). The prep
  command `kk-stream-display.sh start` creates `KK-STREAM` with `hyprctl output create
  headless`, sets the mode with `hyprctl eval "hl.monitor(...)"` (Lua config; a mode sent right
  after create is lost, so the script retries until the output reports it) and focuses it so
  new windows open there; `stop` removes it and gives focus back. Real monitors are not touched.
- Removed: "Desktop (match my screen)" (changed DP-1's own mode) and "Low Res Desktop"
  (xrandr, does nothing on Hyprland). A leftover headless output `SUNSHINE-1` was the first
  monitor, so Sunshine streamed an empty screen; removed. The user unit
  `app-dev.lizardbyte.app.Sunshine.service` (Solar-Flare build in `~/Solar-Flare`, which no
  longer exists) failed at every login; disabled, file kept.
- Network: web UI from this PC only (`origin_web_ui_allowed = pc`, <https://localhost:47990>),
  UPnP off. `firewall.sh` (run with sudo) limits the stream ports to the LAN
  (192.168.178.0/24) and WireGuard (10.8.0.0/24); before, ufw allowed them from anywhere and
  on 2026-10-10 a client connected from a public address (178.230.76.1), so the router
  forwards them: remove those forwards and use WireGuard away from home.

Sources: github.com/ClassicOldSong/Apollo (Linux virtual display discussion #1516),
github.com/Nonary/Vibepollo (README), tech-insider.org "Apollo vs Sunshine 2026",
github.com/alonsojr1980/moonlight-android-turbo (decoder latency, merged into Artemis),
github.com/ImStillBlue/sunshine-virtual-display (headless display for Sunshine on CachyOS).
