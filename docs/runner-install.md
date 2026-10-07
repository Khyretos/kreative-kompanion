# Installing kompanion-runner on a PC

The runner reports the PC's CPU, RAM, disk and GPU stats to Kompanion. It only
makes outbound HTTPS requests; nothing listens on the PC. Linux, x86-64.

## 1. Pair the PC (in the app)

Machines tab → "Pair a computer" → name (e.g. `soucouyant`) → Pair. The app
shows the machine id, the token (only once) and the config below.

## 2. Install the binary

Static binary, no dependencies. Built by CI on kireserver (artifact
`kompanion-runner-x86_64-linux-musl`), or locally with
`cd runner && cargo build --release`.

```bash
install -Dm755 kompanion-runner-x86_64-linux-musl ~/.local/bin/kompanion-runner
```

On soucouyant, `~/Docker` from kireserver is mounted over NFS; the binary is at
`~/Docker/Personal-projects/kreative-kompanion/dist/kompanion-runner-x86_64-linux-musl`.

## 3. Config and token

```bash
mkdir -p ~/.config/kompanion-runner && chmod 700 ~/.config/kompanion-runner
# paste the token from the app into this file:
install -m600 /dev/null ~/.config/kompanion-runner/token && nano ~/.config/kompanion-runner/token
```

`~/.config/kompanion-runner/config.toml`:

```toml
server = "https://kompanion.kreative-kompas.com"
machine_id = "<id shown in the app>"
token_file = "~/.config/kompanion-runner/token"
```

## 4. Run it as a systemd user service

`~/.config/systemd/user/kompanion-runner.service`:

```ini
[Unit]
Description=Kreative Kompanion runner (machine stats)
After=network-online.target

[Service]
ExecStart=%h/.local/bin/kompanion-runner
Restart=always
RestartSec=10

[Install]
WantedBy=default.target
```

```bash
systemctl --user daemon-reload
systemctl --user enable --now kompanion-runner
loginctl enable-linger "$USER"   # keep it running when logged out
journalctl --user -u kompanion-runner -f
```

The PC shows up in the Machines tab within a minute. It reports every 60 s
when nobody looks, at the slider's rate while the tab is open, and every
second on "Live". Exit code 3 means the token was refused: pair again.

## 5. Studio apps on a gaming PC (optional, GPU-01)

On a PC that also runs the studio apps (ComfyUI, music, sound effects), the
runner can start and stop them so the GPU is free for games. Name the apps and
their containers in `config.toml`:

```toml
[gpu_apps]
comfyui = "comfyui-rocm"
heartmula = "heartmula-rocm"
sfx = "moss-sfx-rocm"
```

Then grant "GPU apps" under Machines → Access → System rights. The runner only
runs `docker start`, `docker stop` and `docker inspect` on these containers and
asks the local Ollama to unload its models; nothing else. It also reports when
a game runs (a `gamescope` process, or a Steam game started through
`reaper SteamLaunch`), so Kompanion can switch the apps off by itself.

## 6. The server's own computer (HOST-01)

The computer that runs the Kompanion server can get its runner without copying
code from the web page. After `docker compose up -d` and creating the first
account, run this on the host as your normal user:

```sh
sh tools/install-host.sh --url https://kompanion.example
```

It asks whether Kompanion is your personal app on this PC or a service for
several people and computers, then whether Kompanion may also act on this
computer. Personal installs default to yes, services to no. Yes gets a one-time
code with `docker exec kreative-kompanion kompanion-server pair-host` and runs
the runner installer with it; the runner starts with no rights until you grant
them in Access. No (or `--runner no`) installs nothing.

Use `--mode personal|service`, `--runner yes|no` and `--container <name>` to
skip the questions. The runner pairs under the server's `machine_name` in
`kompanion.toml`. Machines shows ONE card for this computer, marked "this
server", with the server's stats and the runner's grants. An existing pairing
with that name merges the same way. Without a runner the built-in server card
stays. Machines > Pair also offers "This server's computer" with the one
command to run there.

Security note: the container never installs or starts anything on the host by
itself (no Docker socket or host home in the container); this script runs on
the host, by you.

## What it reads

`/proc/stat`, `/proc/meminfo`, `/proc/uptime`, `/proc/loadavg`, the root
filesystem's usage, and per GPU `/sys/class/drm/cardN/device` plus its hwmon
(amdgpu: busy %, VRAM, power, temperature, clocks, fan; Intel i915/xe: clocks,
temperature, power from the energy counter, busy share from idle residency).
No root needed.
