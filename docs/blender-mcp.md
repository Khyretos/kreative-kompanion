# Blender in chats (BLD-01)

Kompanion can build and render simple 3D scenes from a chat with a headless Blender on one of your computers; the render shows in the chat. It is opt-in and off until you install it: a chat that uses it can run any Python inside Blender as the user who installed it, so install it only on a computer where that is fine, and keep the token secret.

## What it is

tools/blender-mcp/server.py is a small MCP server (streamable HTTP, Python standard library only) that starts a fresh `blender -b --factory-startup` for every call, so nothing stays open and no window is needed. Tools: `blender_scene` (a plan, see below), `blender_run` (runs Python with bpy, returns what it prints) and `blender_render` (builds a scene from an empty one; adds a camera aimed at the objects and a sun light when the code adds none; returns a PNG that Kompanion stores with the chat and shows under the tool line, plus the scene as a `.blend` resource that the chat offers as a download so it can be opened and worked on in Blender). tools/blender-mcp/scene.py holds that camera and light logic. Renders use Cycles on the CPU with denoising (OpenImageDenoise) by default, 128 samples (BLENDER_ENGINE=EEVEE for fast drafts, BLENDER_SAMPLES to change); quality over speed. Calls time out after 120 s (BLENDER_TIMEOUT; the container uses 300).

## Scenes that make sense (BLD-03)

The model does not guess coordinates any more; it writes a plan and the server places things so they touch.

- **blender_scene takes a plan**: objects with a name, shape, size, colour (a name such as `red`, or r,g,b) and one relation: `at [x, y]` (stands on the floor), `at [x, y, z]` (in the air, with `floating: true`), `on <name>`, `attached_to <name>` + `side` (front, back, left, right, top, bottom) + `offset`, or `next_to <name>` + `side`. An object may refer only to one listed before it. Shortcut templates: `snowman`, `character`, `table` (more objects can sit `on: "top"`).
- **blender_render keeps raw bpy**, with helpers in the code's namespace: `add`, `put_on_ground`, `put_on`, `stack`, `attach`, `next_to`, `ground_plane`, `face_camera`, `build` (scene.py). A failing script returns the failing line, the error and a hint (errors.py) instead of Blender's log.
- **Measured facts**: every render comes back with the sizes, positions and what touches what (scene.py `layout_report`), plus `WARNING` lines for floating parts and flat sheets standing on edge.
- **Picture check**: when `BLENDER_VISION_URL` (an OpenAI-compatible base URL, e.g. Ollama on soucouyant `http://192.168.178.80:11434/v1`) and `BLENDER_VISION_MODEL` are set, a vision model looks at the render and its answer is appended (vision.py; it only says what and where, touching is measured). Without it the answer says there is no picture check; nothing is ever invented.
- Tests: `python3 tools/blender-mcp/check_errors.py`, `check_vision.py`, and in a real Blender `blender -b --factory-startup --python-exit-code 1 --python tools/blender-mcp/check_build.py`. Live: `python3 tools/blender-mcp/live_chat.py "Render a snowman" out.png`.

## Install

On the computer with Blender (installed as a package, or the Flathub flatpak org.blender.Blender):

```sh
sh tools/blender-mcp/install.sh --bind 192.168.1.20:9876
```

`--bind` is the address Kompanion reaches (default 127.0.0.1:9876, only for Kompanion on the same computer); `--blender "CMD"` picks the Blender command (found automatically). It copies the server to ~/.local/share/kompanion/blender-mcp, writes the token, address and command to ~/.config/kompanion/blender-mcp.env (mode 600; a second run keeps the token) and starts the user service kompanion-blender-mcp with `systemctl --user`. For it to run without you logged in: `loginctl enable-linger $USER`.

### On demand (dormant until used)

`sh tools/blender-mcp/install.sh --on-demand --bind 192.168.1.20:9876` installs a systemd socket (kompanion-blender-mcp.socket) instead of an always-on service. Nothing runs until Kompanion connects; systemd then starts the server, which exits by itself after 5 minutes without requests (BLENDER_IDLE_EXIT).

### In a container

tools/blender-mcp/Dockerfile builds an image with the official Blender (CPU only, no GPU). On kireserver it is Services/blender-mcp (compose, network nginx-reverse-proxy_default, url http://blender-mcp:8000/mcp). Renders of the container are about 7 s for a 960x540 scene.

## Connect Kompanion

Put the token (BLENDER_MCP_TOKEN from the env file) in Kompanion's .env as MCP_BLENDER_TOKEN, add this to kompanion.toml and restart Kompanion:

```toml
[[mcp]]
name = "Blender"
url = "http://192.168.1.20:9876/mcp"
token_env = "MCP_BLENDER_TOKEN"
description = "Headless Blender: builds and renders scenes from a chat"
```

Then Blender appears on the Capabilities page with its two tools, and a chat turns it on in the Tools menu. Example ask: "Render a monkey on a plane."

## Remove

`systemctl --user disable --now kompanion-blender-mcp`, delete ~/.local/share/kompanion/blender-mcp, the env file and the unit file in ~/.config/systemd/user, and remove the [[mcp]] entry.

## Why not the blender-mcp add-on

The popular blender-mcp project (MIT, a Blender add-on plus an MCP server over stdio) drives a Blender window you have open and connected by hand. Kompanion runs on a server and talks MCP over HTTP, so a headless Blender per call fits better: nothing to keep open, and a crash ends with the call. The add-on stays a good choice for working in Blender's window yourself.

## Tests

`python3 tools/blender-mcp/test_server.py` (a fake Blender), `sh tools/blender-mcp/test_install.sh` (a fake HOME and systemctl), `sh tools/blender-mcp/test_install_ondemand.sh` and `python3 tools/blender-mcp/check_ondemand.py` (on-demand mode), `sh tools/blender-mcp/check_scene.sh [ssh host]` (a real Blender).
