// Service worker for Kreative Kompanion: caches the shell on install and serves assets offline, while letting API streams and downloads hit the network.
const CACHE = "kk-shell-v1";
const SHELL = ["./", "index.html", "main.js", "styles.css", "manifest.webmanifest", "icon-192.png", "icon-512.png"];

self.addEventListener("install", (e) => {
  e.waitUntil(
    caches.open(CACHE).then((cache) => cache.addAll(SHELL)).then(() => self.skipWaiting())
  );
});

self.addEventListener("activate", (e) => {
  const currentCaches = caches.keys().then((names) => names.filter((name) => name !== CACHE));
  e.waitUntil(
    currentCaches.then((cachesToDelete) => Promise.all(cachesToDelete.map((cache) => caches.delete(cache)))).then(() => self.clients.claim())
  );
});

self.addEventListener("fetch", (e) => {
  const req = e.request;
  if (req.method !== "GET") return;
  const url = new URL(req.url);
  // The network only: other hosts, the API (live data, event stream) and downloads.
  if (url.origin !== self.location.origin || url.pathname.startsWith("/api/") || url.pathname.startsWith("/download/")
    || (req.headers.get("Accept") || "").includes("text/event-stream")) return;
  // Pages: the network first, a cached copy of the app when offline.
  // Everything else (scripts, styles, icons): the network first, the cache when offline.
  const key = req.mode === "navigate" ? "index.html" : req;
  e.respondWith((async () => {
    try {
      const res = await fetch(req);
      if (res.ok) {
        const copy = res.clone();
        e.waitUntil(caches.open(CACHE).then((c) => c.put(key, copy)));
      }
      return res;
    } catch (err) {
      const cached = await caches.match(key);
      if (cached) return cached;
      throw err;
    }
  })());
});

// NOTIFY: Web Push. The payload is {title, state, url} (task or approval); a click opens or
// focuses Kompanion on that link.
self.addEventListener("push", (e) => {
  let d = {};
  try { d = e.data ? e.data.json() : {}; } catch { d = {}; }
  const title = typeof d.title === "string" && d.title ? `Kompanion: ${d.title.slice(0, 80)}` : "Kompanion";
  const state = typeof d.state === "string" ? d.state : "";
  const body = state === "waits for your approval" ? "A step waits for your approval." : state ? `Now: ${state}.` : "";
  const url = typeof d.url === "string" && d.url.startsWith(self.location.origin) ? d.url : self.location.origin + "/";
  e.waitUntil(self.registration.showNotification(title, { body, icon: "icon-192.png", badge: "icon-192.png", tag: url, data: { url } }));
});

self.addEventListener("notificationclick", (e) => {
  e.notification.close();
  const url = (e.notification.data && e.notification.data.url) || self.location.origin + "/";
  e.waitUntil((async () => {
    const wins = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
    const open = wins.find((w) => w.url.startsWith(self.location.origin));
    if (open) {
      await open.focus();
      if ("navigate" in open) await open.navigate(url).catch(() => undefined);
      return;
    }
    await self.clients.openWindow(url);
  })());
});
