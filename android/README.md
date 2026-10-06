# Kompanion for Android

A small Java app (no AndroidX) that shows the Kompanion web app in a WebView and notifies about tasks; the server address is `server_url` in app/src/main/res/values/values.xml (change `inside_hosts` with it).

## Build

- `sh keystore.sh` once (release key outside the repo, in ~/.config/kompanion-android), then `sh build.sh`; the APK lands in `dist/kompanion.apk`. Builds run in Docker (Dockerfile.build); no Android Studio needed.

## Notifications

Two paths, the simple one first:
1. Own connection (default, no setup): with no UnifiedPush distributor installed, LiveService keeps the server's `/api/events` stream open with the app's sign-in cookie and turns `notify` events into notifications. A foreground service of type remoteMessaging (dataSync is stopped after 6 hours a day on Android 15) with one quiet ongoing notification; reconnects with backoff (2 s up to 5 min) and right away when the network comes back; starts again after a reboot. Switch: the app's notification settings (long-press the app icon > Notification settings, or tap the ongoing notification).
2. UnifiedPush (optional): with a distributor such as ntfy installed, the app registers through it (POST /api/push/register) and stops its own connection.
Phones that stop background apps (Xiaomi, Huawei, Oppo, Vivo ...): the settings screen opens once on first start with a button to the maker's autostart page (Xiaomi HyperOS: Security > Autostart).

## Manual check list

A numbered list to run on a real phone before a release:
1. Uninstall ntfy. Install the APK, sign in through Keycloak; the "Kompanion is connected" notification appears.
2. Close the app (swipe it away), lock the phone. Run the W2 demo task to "needs you": a notification arrives within a minute; tapping it opens that task.
3. Turn on airplane mode for a minute, then off: the next task change still notifies.
4. Reboot the phone without opening the app: the connected notification comes back and notifications still arrive.
5. Sign out in the web app: the ongoing notification says "Open Kompanion to sign in again"; signing in brings back "Kompanion is connected".
6. Switch "Stay connected for notifications" off: the ongoing notification goes away and no task notifications arrive.
7. On a Xiaomi phone, fresh install: the settings screen opens once; "Open settings" opens the Autostart page; it does not open again on the next start.
8. Install ntfy pointed at your server and open the app: it registers for push, the connected notification goes away, and a "needs you" task notifies through ntfy.
9. Check light and dark system themes and the Spanish system language (texts are translated).
