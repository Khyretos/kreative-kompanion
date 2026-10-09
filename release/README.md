# Releases (REL-01)

Push a tag `vX.Y.Z` to `main` on Forgejo and `.forgejo/workflows/release.yml` builds every
package on the soucouyant runner and publishes them as a Forgejo release, the same way
3dco+ does: one Docker builder image per platform, the files copied out with `docker cp`,
then `forgejo-release`. "Run workflow" on the Actions page builds everything without a
release (version `0.0.<run>`, files on the run's Artifacts).

| Platform | Files | Builder |
| --- | --- | --- |
| Linux | `…-linux-x86_64.tar.gz` (plain binary), `…-x86_64.AppImage`, `…-amd64.deb` | `Dockerfile.linux` (Debian 12, Tauri CLI) |
| Windows | `…-x64-setup.exe` (NSIS), `…-x64-portable.exe`, `…-x64.msi` | `Dockerfile.windows` (cargo-xwin, NSIS, wixl from msitools; `windows/kompanion.wxs`) |
| macOS | `…-macos-universal.zip` (Intel + Apple Silicon `.app`) | `Dockerfile.macos` (osxcross, `macos/Info.plist`) |
| Android | `kreative-kompanion.apk` | `Dockerfile.android` on top of `android/Dockerfile.build` |

`SHA256SUMS` lists every file. Release notes: `release/notes/vX.Y.Z.md` (optional).

Every app starts with a server picker (client mode): it suggests
`KOMPANION_DEFAULT_SERVER` from the workflow, and anyone can type their own server.

## Secrets (Android)

The APK is signed with the same key as `android/build.sh`, so it updates an installed app.
Repository Settings > Actions > Secrets:

- `KK_ANDROID_KEYSTORE`: `base64 -w0 ~/.config/kompanion-android/release.jks`
- `KK_ANDROID_KEYSTORE_ENV`: the contents of `~/.config/kompanion-android/keystore.env`

They reach the build as BuildKit secrets and never land in an image layer. Without them
the Android job only warns and the release goes out without the APK.

## Notes per platform

- **Windows:** WiX only runs on Windows, so the .msi comes from wixl (a WiX 3 subset).
  Windows 10 and 11 ship the WebView2 runtime; the setup .exe installs it when missing.
  Not code-signed: SmartScreen asks once ("More info" > "Run anyway").
- **macOS:** Tauri's bundler only runs on macOS, so the `.app` is assembled by hand. It is
  not notarized: the first start needs right-click > Open (or
  `xattr -dr com.apple.quarantine "/Applications/Kreative Kompanion.app"`).
- **Linux:** the plain binary needs WebKitGTK 4.1 (`libwebkit2gtk-4.1-0`); the AppImage
  bundles it.

## Building one platform by hand

```sh
docker build -f release/Dockerfile.linux --build-arg APP_VERSION=0.2.0 -t kk-rel-linux .
docker run --rm -v "$PWD/dist:/dist" kk-rel-linux
```
