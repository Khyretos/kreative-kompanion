<p align="center">
  <img src="docs/branding/banner.png" alt="Kreative Kompanion: your AI, your machines" width="100%">
</p>

# Kreative Kompanion

A self-hosted, FOSS companion. You talk to one orchestrator per project; tasks run on your own computers; local models do the work and a stronger model reviews it and writes lessons into the skills. See `docs/architecture.md`.

## Screenshots

| | |
|---|---|
| ![Sign-in with single sign-on](docs/screenshots/01-sign-in.png) | ![Connect to a server](docs/screenshots/02-connect.png) |
| Sign-in with single sign-on (Keycloak or any OIDC provider) | Connect by link or find the server on your network |
| ![Dashboard](docs/screenshots/03-dashboard.png) | ![Task](docs/screenshots/04-task.png) |
| Projects, the orchestrator chat and the task board | A task: runs, review state, and where it runs |
| ![Run steps](docs/screenshots/05-run-steps.png) | ![Machines](docs/screenshots/06-machines.png) |
| A task running step by step on a computer | CPU, RAM, GPU load and watts of every machine |
| ![Access](docs/screenshots/07-access.png) | ![Activity](docs/screenshots/08-activity.png) |
| Access grants per computer and folder | Activity: every step and grant, per computer |
| ![Capabilities](docs/screenshots/09-capabilities.png) | ![Assets](docs/screenshots/10-assets.png) |
| Capabilities: models, GPUs and the GPU timeline | The asset library with previews and AI tags |
| ![Games](docs/screenshots/11-games.png) | ![Search](docs/screenshots/12-search.png) |
| Games: what a game needs, picked from the library | One search for tasks, chats, projects, assets and settings (Ctrl K) |
| ![Models and roles](docs/screenshots/13-models-and-roles.png) | ![Light theme](docs/screenshots/14-light-theme.png) |
| Any model in any role: local, DeepSeek, Claude | Kompas Day theme |
| <img src="docs/screenshots/15-phone.png" alt="Phone" width="45%"> | |
| On a phone | |

Screens other than sign-in show demo data. `docs/screenshots/shoot.mjs` takes them again (see the comment at its top).
The banner source is `docs/branding/banner.html`; `docs/branding/render-banner.mjs` renders it.

## What works today

- Chat with any OpenAI-compatible or Anthropic model, streaming, a full log of every model call; roles (orchestrator, worker, reviewer and teacher) per project.
- Sign-in with password or OIDC single sign-on; new people get an account on their first SSO sign-in; separate data per user.
- Runner on each PC (`runner/`): pairing with a one-time code, CPU/RAM/disk/GPU stats, tools limited to the folders and time you grant (Access tab), with approval cards for anything else.
- Tasks run by themselves on a computer: a plan with a "done when", steps under your grants, a check command, review and up to three fix rounds; progress live in the task's own chat.
- Capabilities page: models, computers, tools, MCP servers, indexes and skills, live.
- Voice: push-to-talk with Whisper and replies read aloud with Kokoro, both on your own server; nothing is stored.
- Asset library: index of your game assets with previews, AI tags, packs, licences, and per game the assets it needs and uses.
- Global search (Ctrl K), desktop notifications and mail when a task needs you, failed or is done.
- Installable as an app (PWA) on phones and desktops.
- GPU scheduling (milestone 6, phase A): a live ledger of what each GPU holds, jobs that reserve VRAM with priorities (chat, then code, then assets) and night batches, automatic coder/artist switching of the A770 (`gpu-role/`), and a timeline of VRAM, watts and jobs per GPU.
- Studio workflows (M6-05): saved ComfyUI graphs in `studio/workflows/<name>/` (`workflow.toml`: parameters mapped to node inputs, VRAM per machine, every model with its licence). A licence other than Apache-2.0, MIT, BSD, CC0 or CC-BY with attribution shows a warning; it never blocks a run. The visual novel types (OC sheet, VN portrait, VN scene) take a rating (general, sensitive, questionable, explicit, a `choice` parameter); questionable and explicit need the adult-content right: `kompanion-server user-adult <user name> on` (off takes it away). OC sheet and VN portrait also take an optional face photo for likeness (`[face]` in `workflow.toml`: `graph-face.json` with IP-Adapter plus-face; JPEG, PNG or WebP up to 12 MB as `face` in a multipart `POST /api/studio/make`, weight `face_weight` 0 to 1.2, default 0.85); the server keeps it only until the run ends, and ComfyUI gets it in its input folder under `kompanion/`. A run is a GPU job (`POST /api/studio/workflows/<name>/run {gpu, params}`, admins, or `kompanion-server studio-run <name> <gpu> '{"seed": 42}'`), its outputs land in `[studio] output_dir` with a `<file>.json` provenance next to each. To write them to a host folder, set `KOMPANION_GENERATED_DIR` (and `KOMPANION_GENERATED_GID`, the folder's group) in `.env` and `output_dir = "/generated/<sub>"`.

- A Linux desktop app (Tauri 2, `desktop/`): an AppImage and a .deb from CI.
In progress: the game studio (milestone 6, phases B to D). The plan is in `docs/plans/m6-tasks.md`.

## Parts of the repo

- `server/`: Rust (axum, SQLite), the server and web API. Licence GPL-3.0-or-later.
- `web/`: the web app, vanilla TypeScript.
- `desktop/`: the Linux desktop app (Tauri 2).
- `runner/`: the runner for each PC. Licence MIT. Install: `docs/runner-install.md`.
- `machine-stats/`: CPU, RAM and GPU readings shared by server and runner.
- `gpu-helper/` and `gpu-role/`: small services that let the server read and switch the GPUs (`gpu-role/README.md`).
- `skills/`: lessons per role that the models read (`skills/README.md`).
- `tools/`: `deploy.sh` and the drafting pipeline for the local models (`tools/qwen/pipeline.py`).

## Run it

```sh
cp server/kompanion.example.toml kompanion.toml   # edit providers and roles
cp .env.example .env                              # API keys, if any
docker compose up -d
docker compose logs kompanion   # shows the one-time setup code
```

Then open the app, enter the setup code and create your account.

Optional, on the same computer: `sh tools/install-host.sh --url https://kompanion.example` lets Kompanion act on this computer too (it asks first; the runner has no rights until you grant them). See docs/runner-install.md.

For development: `cd web && npm run build`, then
`cd server && KOMPANION_CONFIG=../kompanion.toml cargo run` (set
`web_dir = "../web/dist"` and, for plain http on localhost only,
`secure_cookies = false`).

## Phone app (Android)

- `android/` holds the Android app (a WebView around the web app); build and checks in `android/README.md`.
- Notifications work without any setup: after you sign in, the app keeps its own connection to the server and shows a notification when a task needs you, failed or is done (as set in Settings > Notifications). It shows one quiet "Kompanion is connected" notification so the phone does not close it. You can switch it off in the app's notification settings (long-press the app icon).
- Some phones (Xiaomi, Huawei, Oppo, Vivo and others) stop apps in the background. On those, the app opens a settings screen once with a button to the right page: allow Autostart and set Battery to No restrictions.
- Optional, for technical users: push through UnifiedPush with your own ntfy server, which saves a little battery. Set `KOMPANION_NTFY_URL=https://ntfy.example.com` in `.env`, run `docker compose --profile push up -d` (this also writes the address into `[push] servers` in `kompanion.toml`), put your HTTPS reverse proxy in front of port 8081, and install the ntfy app from F-Droid pointed at that server. The Kompanion app then uses push and closes its own connection.

## Server security

- No open endpoints except status, setup and sign-in. The first account needs a
  one-time setup code printed in the server log.
- Session cookie: random token (stored only as a SHA-256 hash), HttpOnly,
  Secure, SameSite=Strict. Passwords hashed with Argon2id; sign-in throttled.
- State-changing requests need an `X-Kompanion: 1` header and an allowed Origin.
- Strict Content-Security-Policy with Trusted Types, `nosniff`, `no-referrer`.
- Provider URLs come only from the admin's config file (no user-supplied URLs,
  so no SSRF through the API). API keys come from environment variables and are
  never stored or logged.
- The container runs as a non-root user with a read-only filesystem and no
  capabilities.

## Web app

Vanilla TypeScript, no framework. Built with esbuild; two small runtime
dependencies: `marked` (markdown) and `DOMPurify` (sanitising model output).

```sh
cd web
npm install
npm run dev        # http://localhost:5173
npm run typecheck
npm run build      # dist/
node build.mjs --single   # dist/preview.html, one self-contained file
```

Layout of `web/src`:

- `api/` types shared with the server, the `KompanionApi` interface, and the mock server
- `core/` small building blocks: escaped templates (`html`), sanitised markdown,
  a store that batches updates per animation frame, keyed list updates
- `views/` one file per screen part: connect, sign-in, sidebar, conversation, tasks, machines, access, activity, capabilities, assets, games, search, settings

Security rules the code follows: model text is never put into the page as raw
HTML (marked + DOMPurify, links forced to http(s) and `noopener`), templates
escape every value, a strict CSP with Trusted Types is set in `index.html`, and
approvals show the exact command, folder, machine and network targets.
