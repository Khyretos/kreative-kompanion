// CTX-01: the context meter in the chat's top bar: a ring that fills with the chat's share of the
// model's window and shifts from green through orange to red (it glows when nearly full), the share
// in its centre; its details show the numbers, the summary of the older messages and "Summarise now".
import { html, type SafeHtml } from "../core/html";
import { relTime } from "../core/time";
import { num, t } from "../core/i18n";
import type { ChatContext } from "../api/types";

/** Share of the window at which the meter warns (the server summarises at the same point). */
export const WARN = 0.8;
const FULL = 0.95;
/** The ring: radius 15.5 in a 36 x 36 box. */
const R = 15.5;
const C = 2 * Math.PI * R;

/** 104300 -> "104k", 1250 -> "1.3k", 1000000 -> "1M". */
export function tokens(n: number): string {
  if (n >= 1_000_000) return `${+(n / 1_000_000).toFixed(n % 1_000_000 ? 1 : 0)}M`;
  if (n >= 10_000) return `${Math.round(n / 1000)}k`;
  if (n >= 1000) return `${+(n / 1000).toFixed(1)}k`;
  return String(n);
}

export function level(c: ChatContext): "ok" | "warn" | "full" | "unknown" {
  if (c.used === null || !c.window) return "unknown";
  const r = c.used / c.window;
  return r >= FULL ? "full" : r >= WARN ? "warn" : "ok";
}

/** The arc's hue for a share in percent: green (145) to orange (32) at the warning, red (0) when full. */
export function hue(pct: number): number {
  const stops: [number, number][] = [[0, 145], [60, 105], [80, 32], [95, 2], [100, 0]];
  for (let i = 1; i < stops.length; i++) {
    const [p1, h1] = stops[i];
    const [p0, h0] = stops[i - 1];
    if (pct <= p1) return Math.round(h0 + ((pct - p0) / (p1 - p0)) * (h1 - h0));
  }
  return 0;
}

function ring(pct: number | null, centre: string): SafeHtml {
  const len = pct === null ? 0 : (Math.min(pct, 100) / 100) * C;
  // A notch where the warning starts (80%), measured from the top, clockwise.
  const a = (WARN * 360 - 90) * (Math.PI / 180);
  const [nx, ny] = [18 + R * Math.cos(a), 18 + R * Math.sin(a)];
  return html`<svg class="ctx-ring" viewBox="0 0 36 36" aria-hidden="true" style="--ctx-h:${pct === null ? 270 : hue(pct)}">
    <circle class="ctx-track${pct === null ? " unknown" : ""}" cx="18" cy="18" r="${R}"></circle>
    ${pct === null ? "" : html`<circle class="ctx-arc" cx="18" cy="18" r="${R}" stroke-dasharray="${len.toFixed(2)} ${C.toFixed(2)}" transform="rotate(-90 18 18)"></circle>
    <circle class="ctx-notch" cx="${nx.toFixed(2)}" cy="${ny.toFixed(2)}" r="1.3"></circle>`}
    <text class="ctx-centre" x="18" y="18" dominant-baseline="central" text-anchor="middle">${centre}</text>
  </svg>`;
}

/** The meter (nothing before the chat's first answer) and, when open, its details. */
export function contextChip(c: ChatContext | undefined, open: boolean, compacting: boolean, summaryOpen = false): SafeHtml {
  if (!c || (c.used === null && !c.summary)) return html``;
  const lv = level(c);
  const pct = c.used !== null && c.window ? Math.min(100, Math.round((c.used / c.window) * 100)) : null;
  const centre = c.used === null ? "–" : pct !== null ? String(pct) : tokens(c.used);
  const label = c.used === null ? t("ctxNone")
    : pct !== null ? t("ctxFull", { pct, used: num(c.used), window: num(c.window!) })
    : t("ctxUnknown", { used: num(c.used) });
  return html`<span class="ctx">
    <button class="icon-btn ctx-chip" type="button" data-action="context-toggle" data-level="${lv}" aria-haspopup="dialog"
      aria-expanded="${open ? "true" : "false"}" aria-label="${label}" title="${label}">${ring(pct, centre)}</button>
    ${open ? details(c, pct, lv, compacting, summaryOpen) : ""}
  </span>`;
}

function details(c: ChatContext, pct: number | null, lv: string, compacting: boolean, summaryOpen: boolean): SafeHtml {
  const head = c.used === null ? t("ctxNoAnswer")
    : pct !== null ? t("ctxHead", { used: num(c.used), window: num(c.window!), pct })
    : t("ctxHeadUnknown", { used: num(c.used), model: c.model ?? t("ctxThis") });
  const s = c.summary;
  return html`<div class="menu ctx-pop" role="dialog" aria-label="${t("ctxDialog")}">
    <p class="ctx-head">${head}</p>
    ${pct !== null ? html`<span class="ctx-scale" aria-hidden="true" style="--ctx-h:${hue(pct)}"><span class="ctx-scale-fill" style="width:${pct}%"></span><span class="ctx-scale-warn"></span></span>` : ""}
    ${c.model ? html`<p class="ctx-model">${t("ctxModel", { model: c.model })}</p>` : ""}
    ${lv === "warn" || lv === "full" ? html`<p class="ctx-warn" role="status">${t("ctxWarn")}</p>` : ""}
    ${s ? html`<details class="ctx-sum" ${summaryOpen ? "open" : ""}><summary>${s.messages === 1 ? t("ctxSummaryOne", { when: relTime(s.at) }) : t("ctxSummary", { n: s.messages, when: relTime(s.at) })}</summary>
      <p class="ctx-sum-text">${s.text}</p></details>` : ""}
    <p class="ctx-note">${t("ctxNote")}</p>
    <button class="btn small" type="button" data-action="context-compact" ${compacting ? "disabled" : ""}>${compacting ? t("ctxCompacting") : t("ctxCompact")}</button>
  </div>`;
}
