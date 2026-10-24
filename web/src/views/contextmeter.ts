// CTX-01: the context meter in the composer: how much of the model's window the chat uses now,
// a warning near the limit, and the summary of the older messages (with "Summarise now").
import { html, type SafeHtml } from "../core/html";
import { relTime } from "../core/time";
import type { ChatContext } from "../api/types";

/** Share of the window at which the meter warns (the server summarises at the same point). */
export const WARN = 0.8;
const FULL = 0.95;

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

/** The chip (nothing before the chat's first answer) and, when open, its details. */
export function contextChip(c: ChatContext | undefined, open: boolean, compacting: boolean): SafeHtml {
  if (!c || (c.used === null && !c.summary)) return html``;
  const lv = level(c);
  const pct = c.used !== null && c.window ? Math.min(100, Math.round((c.used / c.window) * 100)) : null;
  const short = c.used === null ? "–" : c.window ? `${tokens(c.used)}/${tokens(c.window)}` : `${tokens(c.used)}`;
  const label = c.used === null
    ? "Context: no answer yet. Show details"
    : pct !== null ? `Context ${pct}% full: ${c.used.toLocaleString()} of ${c.window!.toLocaleString()} tokens. Show details`
    : `Context: ${c.used.toLocaleString()} tokens used, the model's window is unknown. Show details`;
  return html`<span class="ctx">
    <button class="chip ctx-chip" type="button" data-action="context-toggle" data-level="${lv}" aria-haspopup="dialog"
      aria-expanded="${open ? "true" : "false"}" aria-label="${label}" title="${label}">
      ${pct !== null ? html`<span class="ctx-bar" aria-hidden="true"><span class="ctx-fill" style="width:${pct}%"></span></span>` : ""}
      <span class="ctx-num">${short}</span></button>
    ${open ? details(c, pct, lv, compacting) : ""}
  </span>`;
}

function details(c: ChatContext, pct: number | null, lv: string, compacting: boolean): SafeHtml {
  const head = c.used === null
    ? "No answer yet in this chat."
    : pct !== null ? `${c.used.toLocaleString()} of ${c.window!.toLocaleString()} tokens (${pct}%)`
    : `${c.used.toLocaleString()} tokens; the window of ${c.model ?? "this model"} is unknown (set context_window for its provider).`;
  return html`<div class="menu ctx-pop" role="dialog" aria-label="Context of this chat">
    <p class="ctx-head">${head}</p>
    ${c.model ? html`<p class="ctx-model">${c.model}, latest answer</p>` : ""}
    ${lv === "warn" || lv === "full" ? html`<p class="ctx-warn" role="status">Nearly full: the next answer first summarises the older messages.</p>` : ""}
    ${c.summary ? html`<details class="ctx-sum"><summary>Summary of ${c.summary.messages} older message${c.summary.messages === 1 ? "" : "s"} · ${relTime(c.summary.at)}</summary>
      <p class="ctx-sum-text">${c.summary.text}</p></details>` : ""}
    <p class="ctx-note">Messages that no longer fit are summarised, never dropped silently.</p>
    <button class="btn small" type="button" data-action="context-compact" ${compacting ? "disabled" : ""}>${compacting ? "Summarising…" : "Summarise now"}</button>
  </div>`;
}
