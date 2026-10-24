// NOTIFY: Web Push in this browser: a subscription with the server's VAPID key, so task and
// approval notifications arrive also when Kompanion is closed (the service worker shows them).
import type { KompanionApi } from "../api/client";

export type PushState = "on" | "off" | "denied" | "unsupported";

function supported(): boolean {
  return "serviceWorker" in navigator && "PushManager" in window && "Notification" in window;
}

async function registration(): Promise<ServiceWorkerRegistration | undefined> {
  if (!supported()) return undefined;
  return (await navigator.serviceWorker.getRegistration()) ?? undefined;
}

/** Base64url to bytes (the server's public key). */
function bytes(b64url: string): Uint8Array<ArrayBuffer> {
  const s = atob(b64url.replace(/-/g, "+").replace(/_/g, "/") + "===".slice((b64url.length + 3) % 4));
  const out = new Uint8Array(new ArrayBuffer(s.length));
  for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i);
  return out;
}

/** A short name for this browser, so a list of devices reads well. */
function device(): string {
  const ua = navigator.userAgent;
  const browser = /Edg\//.test(ua) ? "Edge" : /Firefox\//.test(ua) ? "Firefox" : /Chrome\//.test(ua) ? "Chrome" : /Safari\//.test(ua) ? "Safari" : "Browser";
  const os = /Android/.test(ua) ? "Android" : /iPhone|iPad/.test(ua) ? "iOS" : /Windows/.test(ua) ? "Windows" : /Mac OS/.test(ua) ? "macOS" : /Linux/.test(ua) ? "Linux" : "";
  return os ? `${browser} on ${os}` : browser;
}

export async function state(): Promise<PushState> {
  // The demo has no service worker; it shows the switch as off and works like the real one.
  if (!supported()) return "unsupported";
  if (Notification.permission === "denied") return "denied";
  const reg = await registration();
  return (await reg?.pushManager.getSubscription()) ? "on" : "off";
}

export async function turnOn(api: KompanionApi): Promise<PushState> {
  if (!supported()) return "unsupported";
  if ((await Notification.requestPermission()) !== "granted") return "denied";
  const reg = (await registration()) ?? (await navigator.serviceWorker.ready);
  const { key } = await api.webPushKey();
  const sub = (await reg.pushManager.getSubscription()) ?? (await reg.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: bytes(key) }));
  const json = sub.toJSON() as { endpoint: string; keys: { p256dh: string; auth: string } };
  await api.webPushSubscribe({ endpoint: json.endpoint, keys: json.keys, device: device() });
  return "on";
}

export async function turnOff(api: KompanionApi): Promise<PushState> {
  const sub = await (await registration())?.pushManager.getSubscription();
  if (sub) {
    await api.webPushUnsubscribe(sub.endpoint).catch(() => undefined);
    await sub.unsubscribe();
  }
  return supported() ? "off" : "unsupported";
}
