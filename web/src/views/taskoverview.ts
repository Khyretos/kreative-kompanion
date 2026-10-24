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

export interface Counts { running: Task[]; done: number; total: number; needsYou: number }

/** The counts over the tasks of projects that are not archived. */
export function counts(s: AppState): Counts {
  const archived = new Set(s.projects.filter((p) => p.archivedAt).map((p) => p.id));
  const tasks = s.tasks.filter((x) => !archived.has(x.projectId));
  return {
    running: tasks.filter((x) => x.state === "running"),
    done: tasks.filter((x) => x.state === "done").length,
    total: tasks.length,
    needsYou: tasks.filter((x) => x.state === "needs_input" || x.state === "waiting_resources").length,
  };
}

/** The top bar's chip: "2 running · 5/12 done"; a quiet "0/12 done" when nothing runs. */
export function tasksOverviewChip(s: AppState): SafeHtml {
  const c = counts(s);
  const label = t("ovChip", { running: c.running.length, done: c.done, total: c.total });
  return html`<button class="chip tov-chip ${c.running.length ? "busy" : ""}" type="button" data-action="tasks-overview-toggle" id="ov-chip"
    aria-expanded="${s.tasksOverviewOpen ? "true" : "false"}" aria-controls="tasks-overview-slot" title="${label}" aria-label="${label}">
    <span class="tov-dot" aria-hidden="true"></span><span class="tov-run">${c.running.length}</span><span class="tov-sep" aria-hidden="true">·</span><span class="tov-done">${c.done}/${c.total}</span>
  </button>`;
}

function callCard(c: TraceCall): SafeHtml {
  const tokens = c.tokensIn !== null || c.tokensOut !== null ? t("ovTokens", { in: c.tokensIn ?? 0, out: c.tokensOut ?? 0 }) : "";
  return html`<details class="trace-call ${c.error ? "failed" : ""}">
    <summary>
      <span class="trace-head">${icon("spark")}<span class="kind-tag trace-tag">${t("ovCall")}</span><span class="trace-model">${c.model}</span><span class="muted">${c.provider}</span></span>
      <span class="trace-nums">${tokens}${tokens ? " · " : ""}${(c.ms / 1000).toFixed(1)} s</span>
    </summary>
    ${c.error ? html`<p class="small danger-text">${c.error}</p>` : ""}
    <details class="trace-body"><summary>${t("ovPrompt")}</summary><pre>${c.prompt}</pre></details>
    <details class="trace-body"><summary>${t("ovAnswer")}</summary><pre>${c.response}</pre></details>
  </details>`;
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

/** The panel: counts, the running tasks numbered, or one task's live trace. */
export function renderTasksOverview(s: AppState): SafeHtml {
  if (!s.tasksOverviewOpen) return html``;
  const c = counts(s);
  const project = (id: string) => s.projects.find((p) => p.id === id)?.name ?? "";
  const open = s.tasksOverviewTaskId ? c.running.findIndex((x) => x.id === s.tasksOverviewTaskId) : -1;
  const shownTask = open >= 0 ? c.running[open] : s.tasks.find((x) => x.id === s.tasksOverviewTaskId);
  return html`<section class="tasks-overview" role="dialog" aria-label="${t("ovTitle")}">
    <div class="tov-handle" aria-hidden="true"></div>
    <div class="tov-head">
      <h2>${t("ovTitle")}</h2>
      <button class="icon-btn" type="button" data-action="tasks-overview-toggle" aria-label="${t("ovClose")}">${icon("close")}</button>
    </div>
    <dl class="tov-counts">
      <div><dt>${t("ovRunning")}</dt><dd>${c.running.length}</dd></div>
      <div><dt>${t("ovDone")}</dt><dd>${c.done}</dd></div>
      <div><dt>${t("ovTotal")}</dt><dd>${c.total}</dd></div>
      <div class="${c.needsYou ? "attn" : ""}"><dt>${t("ovNeedsYou")}</dt><dd>${c.needsYou}</dd></div>
    </dl>
    ${shownTask ? detail(s, shownTask, open >= 0 ? open + 1 : 0, c.running.length)
      : c.running.length === 0 ? html`<p class="muted tov-none">${t("ovNone")}</p>`
      : html`<ol class="tov-list">${c.running.map((x, i) => html`<li>
          <button class="tov-row" type="button" data-action="tasks-overview-open" data-id="${x.id}" id="ov-row-${x.id}">
            <span class="tov-num">${i + 1}/${c.running.length}</span>
            <span class="tov-main"><span class="tov-title">${x.title}</span><span class="tov-step">${x.step || t("ovStarting")}</span></span>
            <span class="tov-meta">${project(x.projectId)}${x.progress ? html`<span class="tov-pct">${Math.round(x.progress * 100)}%</span>` : ""}</span>
            ${icon("chevron-right")}
          </button></li>`)}</ol>`}
    <p class="muted small tov-free">${t("ovFree")}</p>
  </section>`;
}
