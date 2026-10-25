// COMPACT-UI: how alive a running task is, from what the server records (no DOM, no network).
import type { Task } from "../api/types";

/** Seconds since an ISO time (Infinity when there is none). */
export function ageSeconds(iso: string | null | undefined, now = Date.now()): number {
  const at = iso ? Date.parse(iso) : NaN;
  return Number.isNaN(at) ? Infinity : Math.max(0, (now - at) / 1000);
}

/** "40 s", "12 min", "3 h", "21 d". */
export function shortAge(seconds: number): string {
  if (seconds < 60) return `${Math.round(seconds)} s`;
  if (seconds < 3600) return `${Math.round(seconds / 60)} min`;
  if (seconds < 86_400) return `${Math.round(seconds / 3600)} h`;
  return `${Math.round(seconds / 86_400)} d`;
}

/** How alive a running task is: live (something in the last 30 s), quiet, stalled (a run but
 *  nothing for 10 min), or board (no Kompanion run: in progress elsewhere, or left over). */
export function liveness(x: Task, now = Date.now()): { kind: "live" | "quiet" | "stalled" | "board"; age: number } {
  const age = ageSeconds(x.lastActivity, now);
  if (!x.activeRun) return { kind: "board", age: x.lastActivity ? age : ageSeconds(x.updatedAt, now) };
  return { kind: age <= 30 ? "live" : age < 600 ? "quiet" : "stalled", age };
}

