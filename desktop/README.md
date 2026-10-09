# Kreative Kompanion desktop app (Apps 1)

Tauri 2 (MIT/Apache-2.0). The window loads a Kompanion server's web UI, so the app has no
second UI to maintain.

- **Client mode (REL-01):** the first start asks for the server's address (`fallback/index.html`),
  checks that `/api/status` answers and remembers it in the app's config folder (`server`).
  Tray > "Change server…" asks again. `--server <url>` or `KOMPANION_SERVER` skips the
  question. Release builds suggest `KOMPANION_DEFAULT_SERVER` (set at build time).

- **Tray icon:** Open / Quit; closing the window hides it to the tray.
- **Notifications:** Settings > On this device > "Desktop notifications" in the web app;
  inside this app they go through Tauri's notification plugin, which only the server's
  own pages may use (`src-tauri/capabilities/default.json`).
- **Sign-in:** the same as in the browser (password or Keycloak), kept in the app's own
  cookie store.
- **Build:** releases (Linux, Windows, macOS) come from `.forgejo/workflows/release.yml`,
  see `release/README.md`. CI (`.forgejo/workflows/desktop.yml`) builds an AppImage and a
  .deb on every change. Locally: `cd desktop && npx @tauri-apps/cli@2 build` (needs
  libwebkit2gtk-4.1-dev and Rust).
- **Install on a Linux PC (tested on soucouyant, CachyOS + Hyprland, 2026-10-05):**
  `desktop/install-linux.sh` builds against the system WebKitGTK (webkit2gtk-4.1) and
  installs the app for your user: `~/.local/bin/kreative-kompanion`, icons and an app-menu
  entry. Add `--autostart` to start it at login. A native build like this suits Arch-based
  systems better than the AppImage, which bundles its own libraries. Hyprland: add
  `windowrulev2 = float, class:^(kreative-kompanion)$, title:^(Open|Save)` if file dialogs
  open tiled.
