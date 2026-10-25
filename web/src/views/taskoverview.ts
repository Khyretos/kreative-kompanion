// TASK-OVERVIEW: the running tasks at a glance: a compact chip in the top bar (pulled down on a
// phone) opens a panel with running / done / total, the running ones numbered (1/3, 2/3…), and a
// live drill-down per task: every model call and every step, with what it is doing now. It reads
// what Kompanion records anyway, so it costs no tokens.
import { html, type SafeHtml } from "../core/html";
import { t } from "../core/i18n";
import { renderStepCard } from "./conversation";
import type { AppState } from "../state";
import type { RunTrace, Task, TraceCall } from "../api/types";
import { icon } from "./icons";
import { ageSeconds, liveness, shortAge } from "../core/liveness";

export interface Counts { running: Task[]; active: Task[]; done: Task[]; needsYou: Task[]; all: Task[] }
export type OverviewFilter = "running" | "needs" | "done" | "all";

/** The task lists over projects that are not archived. `active`: a Kompanion run is going now;
 *  a "running" task without one is in progress on a board (Windshift) or left over. */
export function counts(s: AppState): Counts {
  const archived = new Set(s.projects.filter((p) => p.archivedAt).map((p) => p.id));
  const all = s.tasks.filter((x) => !archived.has(x.projectId));
  const running = all.filter((x) => x.state === "running");
  return {
    running: [...running.filter((x) => x.activeRun), ...running.filter((x) => !x.activeRun)],
    active: running.filter((x) => x.activeRun),
    done: all.filter((x) => x.state === "done"),
    needsYou: all.filter((x) => x.state === "needs_input" || x.state === "waiting_resources"),
    all,
  };
}

/** COMPACT-UI: the top bar's chip: a dot and how many tasks Kompanion runs right now; the dot
 *  pulses only when one of them did something in the last 30 s. */
export function tasksOverviewChip(s: AppState): SafeHtml {
  const c = counts(s);
  const live = c.active.some((x) => liveness(x).kind === "live");
  const label = t("ovChip2", { active: c.active.length, board: c.running.length - c.active.length, needs: c.needsYou.length });
  return html`<button class="chip tov-chip ${live ? "live" : c.active.length ? "busy" : ""}" type="button" data-action="tasks-overview-toggle" id="ov-chip"
    aria-expanded="${s.tasksOverviewOpen ? "true" : "false"}" aria-controls="tasks-overview-slot" title="${label}" aria-label="${label}">
    <span class="tov-dot" aria-hidden="true"></span><span class="tov-run">${c.active.length}</span>
  </button>`;
}

function rowLine(x: Task, filter: OverviewFilter): SafeHtml {
  if (filter === "running" || x.state === "running") {
    const l = liveness(x);
    const step = x.step || (l.kind === "board" ? "" : t("ovStarting"));
    const when = l.kind === "live" ? t("ovNowShort") : t(l.kind === "stalled" ? "ovStalled" : l.kind === "board" ? "ovBoard" : "ovAgo", { age: shortAge(l.age) });
    return html`<span class="tov-step"><span class="tov-dot ${l.kind}" aria-hidden="true"></span><span class="tov-when ${l.kind}">${when}</span>${step ? html` · ${step}` : ""}</span>`;
  }
  if (filter === "needs") return html`<span class="tov-step">${x.step || t("ovWaitsForYou")}</span>`;
  return html`<span class="tov-step">${t("ovAgo", { age: shortAge(ageSeconds(x.updatedAt)) })}</span>`;
}

const LIST_MAX = 30;

/** The call cards and their prompt/answer parts the user opened, kept while the trace redraws live. */
export const openTrace = new Set<string>();

function folded(key: string, cls: string, summary: SafeHtml, body: SafeHtml): SafeHtml {
  return html`<details class="${cls}" ${openTrace.has(key) ? "open" : ""}><summary data-action="trace-toggle" data-key="${key}">${summary}</summary>${body}</details>`;
}

function callCard(c: TraceCall): SafeHtml {
  const tokens = c.tokensIn !== null || c.tokensOut !== null ? t("ovTokens", { in: c.tokensIn ?? 0, out: c.tokensOut ?? 0 }) : "";
  return folded(c.id, `trace-call ${c.error ? "failed" : ""}`, html`
      <span class="trace-head">${icon("spark")}<span class="kind-tag trace-tag">${t("ovCall")}</span><span class="trace-model">${c.model}</span><span class="muted">${c.provider}</span></span>
      <span class="trace-nums">${tokens}${tokens ? " · " : ""}${(c.ms / 1000).toFixed(1)} s</span>`, html`
    ${c.error ? html`<p class="small danger-text">${c.error}</p>` : ""}
    ${folded(`${c.id}:prompt`, "trace-body", html`${t("ovPrompt")}`, html`<pre>${c.prompt}</pre>`)}
    ${folded(`${c.id}:answer`, "trace-body", html`${t("ovAnswer")}`, html`<pre>${c.response}</pre>`)}`);
}

function detail(s: AppState, task: Task, n: number, of: number): SafeHtml {
  const tr: RunTrace | undefined = s.tasksOverviewTrace?.run.taskId === task.id ? s.tasksOverviewTrace : undefined;
  return html`<div class="tov-detail">
    <div class="tov-detail-head">
      <button class="btn small" type="button" data-action="tasks-overview-back" id="ov-back">${icon("back")}${t("ovBack")}</button>
      <h3><span class="tov-num">${n}/${of}</span> ${task.title}</h3>
    </div>
    <p class="tov-now"><span class="tov-dot" aria-hidden="true"></span><strong>${t("ovNow")}</strong> ${tr?.nowDoing || task.step || t("ovStarting")}</p>
    ${tr === undefined ? html`<p class="muted small">${s.tasksOverviewTraceError ?? t("ovLoading")}</p>`
      : tr.items.length === 0 ? html`<p class="muted small">${t("ovEmpty")}</p>`
      : html`<div class="tov-trace" aria-live="polite">${tr.items.map((i) => (i.kind === "call" ? callCard(i) : renderStepCard(i, {})))}</div>`}
  </div>`;
}

/** The panel: one row of filter chips, then the chosen list (running ones numbered and live or
 *  not), or one running task's live trace. */
export function renderTasksOverview(s: AppState): SafeHtml {
  if (!s.tasksOverviewOpen) return html``;
  const c = counts(s);
  const filter: OverviewFilter = s.tasksOverviewFilter ?? "running";
  const list = filter === "running" ? c.running : filter === "needs" ? c.needsYou : filter === "done" ? c.done : c.all;
  const sorted = filter === "running" || filter === "needs" ? list : [...list].sort((a, b) => (b.updatedAt ?? "").localeCompare(a.updatedAt ?? ""));
  const shown = sorted.slice(0, LIST_MAX);
  const project = (id: string) => s.projects.find((p) => p.id === id)?.name ?? "";
  const open = s.tasksOverviewTaskId ? c.running.findIndex((x) => x.id === s.tasksOverviewTaskId) : -1;
  const shownTask = open >= 0 ? c.running[open] : undefined;
  const chip = (f: OverviewFilter, n: number, label: string, cls = "") => html`<button class="tov-filter ${cls}" type="button" data-action="tasks-overview-filter" data-filter="${f}" aria-pressed="${filter === f ? "true" : "false"}"><strong>${n}</strong> ${label}</button>`;
  return html`<section class="tasks-overview" role="dialog" aria-label="${t("ovTitle")}">
    <div class="tov-handle" aria-hidden="true"></div>
    <div class="tov-head">
      <div class="tov-filters" role="group" aria-label="${t("ovTitle")}">
        ${chip("running", c.running.length, t("ovRunningLow"), c.active.length ? "has-live" : "")}
        ${chip("needs", c.needsYou.length, t("ovNeedsLow"), c.needsYou.length ? "attn" : "")}
        ${chip("done", c.done.length, t("ovDoneLow"))}
        ${chip("all", c.all.length, t("ovAllLow"))}
      </div>
      <button class="icon-btn" type="button" data-action="tasks-overview-toggle" aria-label="${t("ovClose")}">${icon("close")}</button>
    </div>
    ${shownTask ? detail(s, shownTask, open + 1, c.running.length)
      : shown.length === 0 ? html`<p class="muted tov-none">${t(filter === "running" ? "ovNone" : "ovNothing")}</p>`
      : html`<ol class="tov-list">${shown.map((x, i) => {
          const drill = filter === "running" && x.activeRun;
          return html`<li>
          <button class="tov-row" type="button" data-action="${drill ? "tasks-overview-open" : "tasks-overview-task"}" data-id="${x.id}" id="ov-row-${x.id}">
            ${filter === "running" ? html`<span class="tov-num">${i + 1}/${c.running.length}</span>` : ""}
            <span class="tov-main"><span class="tov-title">${x.title}</span>${rowLine(x, filter)}</span>
            <span class="tov-meta">${project(x.projectId)}</span>
            ${icon("chevron-right")}
          </button></li>`;
        })}</ol>${sorted.length > LIST_MAX ? html`<p class="muted small tov-more">${t("ovMore", { n: sorted.length - LIST_MAX })}</p>` : ""}`}
  </section>`;
}
