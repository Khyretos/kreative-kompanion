// Middle pane: the conversation with the project's orchestrator.
import { codeSpans, html, type SafeHtml } from "../core/html";
import { renderMarkdown } from "../core/markdown";
import { langOfPath, type Artifact } from "../core/artifact";
import { linkSources, renderSources, splitSources } from "../core/sources";
import { renderOverseer, splitOverseer } from "../core/overseer";
import { splitFiles } from "../core/attachments";
import { renderSent } from "./attach";
import { clock } from "../core/time";
import { activeChat, activeProject, type AppState } from "../state";
import type { Effort, Message, Task } from "../api/types";
import type { PcAction } from "../api/client";
import { icon } from "./icons";
import { contextChip } from "./contextmeter";
import { tasksOverviewChip } from "./taskoverview";
import { t, type Key } from "../core/i18n";
import { stateLabel } from "./tasks";
import { fromTool, renderOutput } from "../core/output";
import { kindOf, styleOf, type CardStyle } from "../core/cardtypes";
import { commandLang, commandText, RISK_LABEL, riskOf, spawnedRef, spawnOf } from "../core/commandcard";
import { highlightHtml } from "../core/codeblocks";

/** Card colours per action type, set by main.ts from the user's Settings. */
let cardStyle: CardStyle | undefined;
export function setCardStyle(c: CardStyle | undefined): void {
  cardStyle = c;
}

const STEP_LABEL: Record<string, Key> = {
  pending: "stPending",
  approved: "stStarting",
  always: "stStarting",
  granting: "stGranting",
  running: "stRunning",
  denied: "stDenied",
  done: "stDone",
  failed: "stFailed",
  refused: "stRefused",
  stopped: "stStopped",
};

const BUSY = ["approved", "always", "granting", "running"];

/** CMD-01: kept for callers; since COMPACT-UI every command card offers Explain (the click is the opt-in). */
export function setExplainOn(_on: boolean): void {}
/** Explanations by step id (the server keeps them per command, so a re-open costs nothing), and
 *  the steps whose explanation the user opened; both kept across re-renders. */
export const explanations = new Map<string, { text?: string; error?: string }>();
export const openExplains = new Set<string>();

/** Steps the user opened stay open across re-renders (main.ts keeps this in step). */
export const openSteps = new Set<string>();
/** The user's open/closed choice per step group ("N steps on …"), by the group's first step
 *  (a group can move to another message when a reply arrives, so not by message). Without a
 *  choice a group is open only while one of its steps runs; a choice is never overridden. */
export const groupChoice = new Map<string, boolean>();

/** "m:ss" since an ISO time (main.ts's ticker keeps it current between renders). */
export function elapsedText(since: string, now = Date.now()): string {
  const secs = Math.max(0, Math.round((now - Date.parse(since)) / 1000));
  return Number.isNaN(secs) ? "" : `${Math.floor(secs / 60)}:${String(secs % 60).padStart(2, "0")}`;
}

/** CMD-01: a step as a card. Header: icon and type, the time, a risk tag and the state; under it
 *  the summary. Open: the command as highlighted code (scrolls), the output, and "Explain command". */
/** COMPACT-UI: the step's one line of code: a shell command highlighted as bash after a short
 *  prompt with its folder, a file step's path, or the summary for anything else. */
function stepLine(a: PcAction): SafeHtml {
  const tool = a.tool ?? {};
  const name = String(tool.tool ?? "");
  const path = typeof tool.path === "string" ? tool.path : "";
  if (name === "shell") {
    const cwd = typeof tool.cwd === "string" ? tool.cwd : "";
    return html`${cwd ? html`<span class="step-cwd" title="${cwd}">${shortPath(cwd)} $</span> ` : ""}${highlightHtml(String(tool.command ?? ""), "bash")}`;
  }
  if (path) return html`<span class="step-path" title="${path}">${shortPath(path)}</span>`;
  const command = commandText(a.tool);
  return command ? highlightHtml(command, commandLang(a.tool)) : codeSpans(a.summary);
}

/** A long path as its last three parts ("…/runner/src/proc.rs"); the full one is in the title. */
export function shortPath(p: string): string {
  if (p.length <= 28) return p;
  const parts = p.split("/").filter(Boolean);
  return parts.length > 3 ? `…/${parts.slice(-3).join("/")}` : p;
}

/** ARTIFACTS: the file a step wrote or read, for its Open link (kept while the chat shows it). */
const stepFiles = new Map<string, Artifact>();
export const stepArtifact = (id: string): Artifact | undefined => stepFiles.get(id);

function fileOf(a: PcAction): Artifact | undefined {
  const tool = a.tool ?? {};
  const path = typeof tool.path === "string" ? tool.path : "";
  if (!path) return undefined;
  if (tool.tool === "write_file" && typeof tool.content === "string" && tool.append !== true && typeof tool.offset !== "number") {
    return { title: path, lang: langOfPath(path), text: tool.content };
  }
  if (tool.tool === "read_file" && a.state === "done" && a.result) return { title: path, lang: langOfPath(path), text: a.result };
  return undefined;
}

/** CMD-01 / COMPACT-UI: a step as a card: one header row (icon, type and time on the left; Explain,
 *  risk and state on the right), then its command as highlighted code, three lines until opened.
 *  Opened: the whole command, the output and the explanation. */
function renderStep(a: PcAction, machines: Record<string, string>): SafeHtml {
  const busy = BUSY.includes(a.state);
  const kind = styleOf(kindOf(a.tool), cardStyle);
  const risk = riskOf(a.tool);
  const command = commandText(a.tool);
  const spawn = spawnOf(a.tool);
  const made = spawn && a.state === "done" ? spawnedRef(a.result) : undefined;
  const since = a.startedAt ?? a.createdAt;
  const took = !busy && a.endedAt ? elapsedText(since, Date.parse(a.endedAt)) : "";
  const icn = busy ? "spark" : a.state === "done" ? "terminal" : "close";
  const explaining = openExplains.has(a.id);
  const file = fileOf(a);
  if (file) stepFiles.set(a.id, file); else stepFiles.delete(a.id);
  return html`
    <details class="step step-card ${a.state}" data-step="${a.id}" data-state="${a.state}" ${busy || openSteps.has(a.id) || explaining ? "open" : ""}>
      <summary data-action="step-toggle" data-id="${a.id}" title="${a.summary}">
        <span class="step-head">
          ${icon(icn)}<span class="kind-tag" style="--kind:${kind.color}">${spawn ? t(spawn.kind === "task" ? "spawnTask" : "spawnChat") : kind.label}</span>
          ${a.state === "running" ? html`<span class="step-time elapsed" data-since="${since}">${elapsedText(since)}</span>` : took ? html`<span class="step-time" title="${t("cmdTook", { time: took })}">${took}</span>` : ""}
          <span class="step-tags">
            ${file ? html`<button class="step-explain-link step-open-link" type="button" data-action="artifact-step" data-id="${a.id}" title="${t("artFileOpen")}">${t("artOpen")}</button>` : ""}
            ${command && !spawn ? html`<button class="step-explain-link" type="button" data-action="explain-toggle" data-id="${a.id}" aria-expanded="${explaining ? "true" : "false"}">${t("explainShort")}</button>` : ""}
            ${risk ? html`<span class="risk-tag risk-${risk}" title="${t("riskTip")}">${t(RISK_LABEL[risk])}</span>` : ""}
            <span class="chip ${a.state}">${STEP_LABEL[a.state] ? t(STEP_LABEL[a.state]) : a.state}</span>
            ${a.state === "running" ? html`<button class="btn small danger step-stop" type="button" data-action="step-stop" data-id="${a.id}">${t("cmdStop")}</button>` : ""}
          </span>
        </span>
        ${spawn ? html`<span class="step-summary">${spawn.title}</span>` : html`<code class="step-line hljs">${stepLine(a)}</code>`}
      </summary>
      <div class="step-explain-body" data-explain-body="${a.id}" aria-live="polite" ${explaining ? "" : "hidden"}></div>
      ${spawn ? html`<div class="spawn-prompt" role="group" aria-label="${t("spawnPrompt")}">${spawn.prompt}</div>` : ""}
      ${spawn && a.result ? html`<p class="spawn-result small">${a.result}</p>` : a.result ? renderOutput({ ...fromTool(a.tool, a.result, machines[a.machineId]), title: undefined, machine: undefined }) : ""}
      ${made ? html`<p class="spawn-open"><button class="btn small" type="button" data-action="${made.kind === "task" ? "open-task" : "open-chat"}" data-id="${made.id}">${t(made.kind === "task" ? "spawnOpenTask" : "spawnOpenChat")}</button></p>` : ""}
    </details>`;
}

/** TASK-OVERVIEW: a step as the same card the chat shows, for a run's trace. */
export function renderStepCard(a: PcAction, machines: Record<string, string>): SafeHtml {
  return renderStep(a, machines);
}

/** Fills an opened explanation: the text as sanitised markdown, a note while it is written, or the
 *  error with a retry. */
export function fillExplain(body: HTMLElement): void {
  const id = body.dataset.explainBody ?? "";
  const x = explanations.get(id);
  if (!x) {
    body.replaceChildren(Object.assign(document.createElement("p"), { className: "muted small", textContent: t("explaining") }));
  } else if (x.error !== undefined) {
    const p = Object.assign(document.createElement("p"), { className: "small danger-text", textContent: t("explainFailed", { error: x.error }) });
    const retry = Object.assign(document.createElement("button"), { type: "button", className: "btn small", textContent: t("explainRetry") });
    retry.dataset.action = "explain-retry";
    retry.dataset.id = id;
    body.replaceChildren(p, retry);
  } else {
    body.replaceChildren(renderMarkdown(x.text ?? ""));
  }
}

function renderSteps(steps: PcAction[], machines: Record<string, string>): SafeHtml {
  if (steps.length === 0) return html``;
  const list = html`${steps.map((a) => renderStep(a, machines))}`;
  if (steps.length < 3) return html`<div class="steps">${list}</div>`;
  const where = machines[steps[0].machineId] ?? "a computer";
  const running = steps.some((a) => BUSY.includes(a.state));
  const key = steps[0].id;
  const open = groupChoice.get(key) ?? running;
  return html`<details class="steps group" data-group="${key}" ${open ? "open" : ""}>
    <summary data-action="group-toggle" data-id="${key}">${steps.length} steps on ${where}${running ? html` <span class="chip running">running…</span>` : ""}</summary>${list}</details>`;
}

export function renderHeader(s: AppState): SafeHtml {
  const chat = activeChat(s);
  const project = activeProject(s);
  const orchestrator = s.roles.find((r) => r.role === "orchestrator");
  const attention = s.tasks.filter((t) => t.state === "needs_input" || t.state === "waiting_resources").length;
  return html`
    <button class="icon-btn only-phone" data-action="pane" data-pane="left" aria-label="Projects and chats">${icon("menu")}</button>
    <div class="conv-title">
      <span class="eyebrow">${project ? project.name : "Chat"}</span>
      <h1>${chat?.title ?? "New chat"}</h1>
    </div>
    <span class="chip model" title="Orchestrator model">${orchestrator?.modelId ?? ""}</span>
    ${tasksOverviewChip(s)}
    ${chat ? html`<span class="head-tools">
      ${contextChip(s.chatContext && s.chatContext.chatId === chat.id ? s.chatContext : undefined, !!s.contextOpen, !!s.compacting, !!s.contextSummaryOpen)}
      <button class="icon-btn facts-btn" type="button" data-action="open-facts" aria-label="Facts for this chat" title="Facts: what Kompanion remembers for this chat and its project">${icon("bookmark")}</button>
      <a class="icon-btn export-btn" href="/api/chats/${encodeURIComponent(chat.id)}/report?download=1" download aria-label="${t("export")}" title="${t("exportTip")}">${icon("download")}</a>
    </span>` : ""}
    <button class="icon-btn only-narrow tasks-toggle" data-action="pane" data-pane="right" aria-label="Tasks">
      ${icon("tasks")}${attention ? html`<span class="badge attn">${attention}</span>` : ""}
    </button>`;
}

/** A message plus the tasks it created; rebuilt only when one of them changes. */
export interface MessageView {
  id: string;
  m: Message;
  tasks: Task[];
  steps: PcAction[];
  machines: Record<string, string>;
  /** MSG-ACT: the chat's newest message is an answer: it can be regenerated. */
  last: boolean;
}

const cache = new Map<string, MessageView>();

export function messageViews(s: AppState): MessageView[] {
  const machines = Object.fromEntries(s.machines.map((m) => [m.id, m.name]));
  const byMsg = new Map<string, PcAction[]>();

  for (const action of s.pcActions) {
    if (action.state === "pending") continue;
    // Find the last message with at <= action.createdAt
    let bestMsg: Message | undefined = undefined;
    for (const m of s.messages) {
      if (m.at <= action.createdAt) {
        bestMsg = m;
      }
    }
    if (bestMsg) {
      const existing = byMsg.get(bestMsg.id);
      if (!existing) {
        byMsg.set(bestMsg.id, []);
      }
      const current = byMsg.get(bestMsg.id)!;
      // Check if already present to avoid duplicates
      if (!current.some((a) => a.id === action.id)) {
        current.push(action);
      }
    }
  }

  const lastId = s.messages.at(-1)?.id;
  return s.messages.map((m) => {
    const last = m.id === lastId && m.author !== "user";
    const tasks = (m.taskIds ?? []).map((id) => s.tasks.find((t) => t.id === id)).filter((t): t is Task => !!t);
    const steps = byMsg.get(m.id) ?? [];
    const old = cache.get(m.id);
    if (old && old.m === m && old.last === last && old.tasks.length === tasks.length && old.tasks.every((t, i) => t === tasks[i]) && old.steps.length === steps.length && old.steps.every((s, i) => s === steps[i])) return old;
    const view = { id: m.id, m, tasks, steps, machines, last };
    cache.set(m.id, view);
    return view;
  });
}

/** The message opened from search: outlined for a moment, also across re-renders. */
let flashId: string | undefined;
export function flashMessage(id: string | undefined): void { flashId = id; }

/** MSG-ACT: the user's message being edited (main.ts keeps it), and the draft typed so far. */
export let editingMessage: { id: string; draft: string } | undefined;
export function setEditingMessage(x: { id: string; draft: string } | undefined): void { editingMessage = x; }

/** The message's own words, without the files and sources blocks (what Copy and Edit take). */
export function messageText(m: Message): string {
  return splitOverseer(splitSources(splitFiles(m.text).text).text).text.trim();
}

/** MSG-ACT: Copy on every message; Edit and resend on the user's, Regenerate on the newest answer. */
function messageActions(m: Message, last: boolean): SafeHtml {
  if (m.streaming) return html``;
  const btn = (action: string, name: Key, ico: Parameters<typeof icon>[0]) =>
    html`<button class="icon-btn msg-act" type="button" data-action="${action}" data-id="${m.id}" aria-label="${t(name)}" title="${t(name)}">${icon(ico)}</button>`;
  return html`<div class="msg-actions" role="toolbar" aria-label="${t("msgActions")}">
    ${btn("msg-copy", "msgCopy", "copy")}
    ${m.author === "user" ? btn("msg-edit", "msgEdit", "edit") : ""}
    ${last ? btn("msg-regen", "msgRegen", "redo") : ""}
  </div>`;
}

export function renderMessage({ m, tasks, steps, machines, last }: MessageView): SafeHtml {
  const editing = editingMessage?.id === m.id;
  return html`
    <article class="msg ${m.author}${m.id === flashId ? " search-hit" : ""}${editing ? " editing" : ""}" id="msg-${m.id}">
      <header>
        <span class="who">${m.author === "user" ? "You" : assistantName}</span>
        <time datetime="${m.at}">${clock(m.at)}</time>
        ${editing ? "" : messageActions(m, last)}
      </header>
      ${editing ? html`<form class="msg-edit" data-id="${m.id}">
        <label class="sr-only" for="msg-edit-${m.id}">${t("msgEditLabel")}</label>
        <textarea id="msg-edit-${m.id}" class="msg-edit-text" rows="3">${editingMessage?.draft ?? ""}</textarea>
        <p class="muted small">${t("msgEditNote")}</p>
        <div class="msg-edit-buttons">
          <button class="btn small" type="button" data-action="msg-edit-cancel" data-id="${m.id}">${t("msgCancel")}</button>
          <button class="btn small primary" type="submit">${t("msgSend")}</button>
        </div>
      </form>` : html`<div class="body"></div>`}
      ${renderSent(m.chatId, splitFiles(m.text).files)}
      ${renderSteps(steps, machines)}
      ${tasks.length ? html`
        <ul class="msg-tasks">${tasks.map((t) => html`
          <li><button class="task-ref" data-action="open-task" data-id="${t.id}">
            <span class="state-dot s-${t.state}" aria-hidden="true"></span>${t.title}
            <span class="muted">${stateLabel(t.state)}</span>
          </button></li>`)}</ul>` : ""}
    </article>`;
}

/** Fills the message body with sanitised markdown (never via the template). */
export function fillMessage(el: HTMLElement, { m }: MessageView): void {
  for (const x of el.querySelectorAll<HTMLElement>("[data-explain-body]")) if (openExplains.has(x.dataset.explainBody ?? "")) fillExplain(x);
  const body = el.querySelector(".body");
  if (!body) return; // MSG-ACT: being edited, a form instead of the text
  const split = splitSources(splitFiles(m.text).text);
  const { text, proposal } = splitOverseer(split.text);
  body.append(renderMarkdown(linkSources(text)));
  if (proposal && !m.streaming) body.append(renderOverseer(proposal, m.id));
  if (split.sources.length) body.append(renderSources(split.sources));
  if (m.streaming) {
    const caret = document.createElement("span");
    caret.className = "caret";
    caret.setAttribute("aria-hidden", "true");
    (body.lastElementChild ?? body).append(caret);
  }
}

export function renderEmpty(s: AppState): SafeHtml {
  const project = activeProject(s);
  return html`
    <div class="empty">
      <h2>What should we work on${project ? html` in ${project.name}` : ""}?</h2>
      <p class="muted">Describe the goal. I'll plan it, split it into tasks and ask when I need you.</p>
    </div>`;
}

const EFFORTS: { id: Effort; label: Key; hint: Key }[] = [
  { id: "auto", label: "effAuto", hint: "effAutoHint" },
  { id: "low", label: "effLow", hint: "effLowHint" },
  { id: "medium", label: "effMedium", hint: "effMediumHint" },
  { id: "high", label: "effHigh", hint: "effHighHint" },
];

/** What effortChip shows, as a string: the chip is mounted again only when this changes, so live
 * updates elsewhere in a chat neither redraw the open menu nor drop its focus. */
export function effortChipKey(s: AppState): string {
  const role = overseerOn(s) ? "overseer" : "orchestrator";
  const a = s.roles.find((r) => r.role === role);
  return JSON.stringify([activeChat(s)?.effort ?? s.draftEffort ?? "auto", role, a?.providerId, a?.modelId, !!s.effortMenuOpen, !!s.effortModelsOpen,
    s.providers.map((p) => [p.id, p.name, !!p.error, p.models.map((m) => m.id)])]);
}

/** EFF-02: the composer's effort chip ("High ▾") and its picker: the model on top (it can be
 * changed there, for the role that answers this chat), then the four effort levels. */
export function effortChip(s: AppState): SafeHtml {
  const level: Effort = activeChat(s)?.effort ?? s.draftEffort ?? "auto";
  const role = overseerOn(s) ? "overseer" : "orchestrator";
  const assigned = s.roles.find((r) => r.role === role);
  const model = assigned?.modelId ?? "";
  const label = t(EFFORTS.find((e) => e.id === level)?.label ?? "effAuto");
  const models = s.providers.filter((p) => !p.error).flatMap((p) => p.models.map((m) => ({ provider: p, model: m.id })));
  return html`<span class="effort">
    <button class="chip effort-chip" type="button" data-action="effort-menu" aria-haspopup="menu"
      aria-expanded="${s.effortMenuOpen ? "true" : "false"}" aria-label="${t("effChip", { effort: label, model: model || t("effNoModel") })}"
      title="${model}">${label} ▾</button>
    ${s.effortMenuOpen ? html`<div class="menu effort-menu" role="menu" aria-label="${t("effMenu")}">
      <button role="menuitem" class="effort-model" data-action="effort-models" aria-expanded="${s.effortModelsOpen ? "true" : "false"}">
        <span class="effort-hint">${t("effModel")}</span>
        <span class="effort-model-row"><span class="effort-name">${model || t("effNoModel")}</span><span class="effort-change">${t("effChange")} ${s.effortModelsOpen ? "▴" : "▾"}</span></span></button>
      ${s.effortModelsOpen ? html`<div class="effort-models" role="group" aria-label="${t("effModels")}">
        ${models.length ? models.map(({ provider, model: id }) => html`<button role="menuitemradio" class="effort-model-pick"
          aria-checked="${assigned?.providerId === provider.id && id === model ? "true" : "false"}" data-action="effort-model" data-provider="${provider.id}" data-model="${id}" data-role="${role}">
          <span class="effort-name">${id}</span><span class="effort-hint">${provider.name}</span></button>`) : html`<p class="effort-hint effort-note">${t("effNoModels")}</p>`}
        <p class="effort-hint effort-note">${t("effModelNote", { role })}</p>
      </div>` : ""}
      <div class="effort-levels" role="group" aria-label="${t("effMenu")}">
      ${EFFORTS.map((e) => html`<button role="menuitemradio" aria-checked="${e.id === level ? "true" : "false"}" data-action="effort-set" data-effort="${e.id}">
        <span class="effort-name">${t(e.label)}</span><span class="effort-hint">${t(e.hint)}</span></button>`)}
      </div>
    </div>` : ""}
  </span>`;
}

/** OVR-01b: the project of this chat (or of the chat about to be made) keeps the Overseer on; not its thread. */
function projectOverseer(s: AppState): boolean {
  const chat = activeChat(s);
  if (chat?.thread) return false;
  const pid = chat ? chat.projectId : s.activeProjectId;
  return !!s.projects.find((p) => p.id === pid)?.overseer;
}

/** Whether the Overseer answers this chat: its own switch or its project's. */
export function overseerOn(s: AppState): boolean {
  const chat = activeChat(s);
  return projectOverseer(s) || !!(chat ? chat.overseer : s.draftOverseer);
}

/** OVR-01: the Overseer chip: this chat is answered by the Overseer, who sees all projects and tasks. */
export function overseerChip(s: AppState): SafeHtml {
  const chat = activeChat(s);
  if (chat?.thread) return html``;
  const name = s.overseer.name;
  if (projectOverseer(s)) {
    return html`<button class="chip overseer-chip locked" type="button" aria-pressed="true" aria-disabled="true"
      aria-label="${name} on for this whole project" title="${name} is always on in this project: switch it off in the project's panel (Tasks tab).">${icon("eye")}<span class="ov-label">${name}</span></button>`;
  }
  const on = !!(chat ? chat.overseer : s.draftOverseer);
  const tip = on
    ? `${name} is on: answers here see all your projects and running tasks, can plan tasks${s.overseer.interject ? " and add context to running tasks" : ""}. Click to turn off.`
    : `Turn on ${name}: sees all your projects and running tasks, gives you the status and plans new tasks with you.`;
  return html`<button class="chip overseer-chip" type="button" data-action="overseer-toggle" aria-pressed="${on ? "true" : "false"}"
    aria-label="${name} ${on ? "on" : "off"}" title="${tip}">${icon("eye")}<span class="ov-label">${on ? name : ""}</span><span class="sr-only">${on ? "" : `${name} off`}</span></button>`;
}

/** AUTO-01: the automatic-mode chip: this chat runs its steps without asking (locked on when its project does). */
export function autoRunChip(s: AppState): SafeHtml {
  const chat = activeChat(s);
  const pid = chat ? chat.projectId : s.activeProjectId;
  if (s.projects.find((p) => p.id === pid)?.autoRun) {
    return html`<button class="chip autorun-chip locked" type="button" aria-pressed="true" aria-disabled="true"
      aria-label="Automatic mode on for this whole project" title="Steps run without asking in this project: switch it off in the project's panel.">${icon("spark")}<span class="sr-only">Automatic mode on for this project</span></button>`;
  }
  const on = !!(chat ? chat.autoRun : s.draftAutoRun);
  const tip = on
    ? `Automatic mode is on: I run steps without asking and without a step limit; Stop ends it at any step. Click to turn off.`
    : "Automatic mode is off: I ask before each step. Click to run steps without asking.";
  return html`<button class="chip autorun-chip" type="button" data-action="autorun-toggle" aria-pressed="${on ? "true" : "false"}"
    aria-label="Automatic mode ${on ? "on" : "off"}" title="${tip}">${icon("spark")}<span class="sr-only">Automatic mode ${on ? "on" : "off"}</span></button>`;
}

/** OVR-01: who wrote a message: the Overseer's name in an Overseer chat. */
let assistantName = "Kompanion";
export function setAssistantName(name: string): void { assistantName = name; }

const WEB = "Web";
const TOOL_HINTS: Record<string, string> = {
  Knowledge: "Your document collections (Capabilities > Knowledge, plus the Open WebUI ones): I search them and cite the file I used.",
  History: "Your past chats: I search them and read back what was said, also in this chat beyond what I remember.",
  Memory: "I save facts worth keeping (paths, commands, decisions) and delete my wrong ones. I always read the Facts tab.",
};
/** HIST-01: built-in tools shown under a friendlier name than their server name. */
const TOOL_LABELS: Record<string, string> = { History: "Past chats", Memory: "Remember facts" };

/** UI-02: Web is a core feature, so it is its own chip; shown only when a search engine is set up. */
export function webChip(s: AppState): SafeHtml {
  if (!s.mcpServers.includes(WEB)) return html``;
  const chat = activeChat(s);
  const on = ((chat ? chat.mcp : s.draftMcp) ?? []).includes(WEB);
  return html`<button class="chip web-chip" type="button" data-action="mcp-toggle" data-name="${WEB}" aria-pressed="${on ? "true" : "false"}"
    aria-label="Web search ${on ? "on" : "off"}"
    title="Web search is ${on ? "on" : "off"}: click to turn it ${on ? "off" : "on"}. When on, I search the web and read pages before answering and cite my sources.">${icon("globe")}<span class="sr-only">Web search ${on ? "on" : "off"}</span></button>`;
}

/** CHAT-01: the Tools chip and its menu (MCP servers this chat may use, each with a short description; Web has its own chip). */
export function toolsChip(s: AppState): SafeHtml {
  const names = s.mcpServers.filter((n) => n !== WEB);
  if (names.length === 0) return html``;
  const chat = activeChat(s);
  const on = ((chat ? chat.mcp : s.draftMcp) ?? []).filter((n) => n !== WEB);
  const tip = on.length ? `${on.length} tool${on.length === 1 ? "" : "s"} on for this chat: click to choose` : "No tools on for this chat: click to choose";
  return html`<span class="tools">
    <button class="chip tools-chip${on.length ? " has-tools" : ""}" type="button" data-action="tools-menu" aria-haspopup="menu" aria-expanded="${s.toolsMenuOpen ? "true" : "false"}" aria-label="${tip}" title="${tip}">${icon("wrench")}<span class="tools-count">${on.length}</span></button>
    ${s.toolsMenuOpen ? html`<div class="menu tools-menu" role="menu" aria-label="Tools for this chat">
      ${names.map((name) => {
        const hint = TOOL_HINTS[name] ?? s.mcpInfo[name] ?? `Tools from the ${name} server, added to this chat.`;
        return html`<button role="menuitemcheckbox" aria-checked="${on.includes(name) ? "true" : "false"}" data-action="mcp-toggle" data-name="${name}" title="${hint}">
          <span class="effort-name">${TOOL_LABELS[name] ?? name}</span><span class="effort-hint">${hint}</span></button>`;
      })}
    </div>` : ""}
  </span>`;
}

export function composer(): SafeHtml {
  return html`
    <div class="attach-row" id="attach-row" aria-live="polite"></div>
    <div class="slash-menu" id="slash-menu" role="listbox" aria-label="Commands" hidden></div>
    <form class="composer" id="composer">
      <input type="file" id="attach-input" multiple hidden>
      <input type="file" id="attach-camera" hidden accept="image/*" capture="environment">
      <span class="attach-wrap">
        <button class="btn attach" type="button" id="attach-btn" aria-haspopup="menu" aria-expanded="false" aria-controls="attach-menu" aria-label="Add to the message">${icon("paperclip")}</button>
        <span class="attach-menu" id="attach-menu" role="menu" hidden>
          <button type="button" role="menuitem" data-attach="files">${icon("file")}<span>Files or pictures</span></button>
          <button type="button" role="menuitem" data-attach="camera">${icon("camera")}<span>Take a photo</span></button>
          <button type="button" role="menuitem" data-attach="screenshot">${icon("screen")}<span>Screenshot of a window or screen</span></button>
          <button type="button" role="menuitem" data-attach="clipboard">${icon("clipboard")}<span>Paste from the clipboard</span></button>
        </span>
      </span>
      <label class="sr-only" for="prompt">Message</label>
      <div class="prompt-box"><textarea id="prompt" rows="1" placeholder="Ask, plan, or hand over a task…"></textarea></div>
      <button class="btn mic" type="button" id="voice-mic" data-action="voice-mic" aria-label="Speak" aria-pressed="false" hidden>${icon("mic")}</button>
      <button class="btn stop-chat" type="button" id="stop-chat" data-action="stop-chat" aria-label="Stop the reply" hidden>Stop</button>
      <button class="btn primary send" type="submit" aria-label="Send">${icon("send")}</button>
      <p class="composer-readonly" hidden>Read-only: this project is shared with you, so you can read this chat but not write in it.</p>
    </form>
    <p class="composer-hint"><span class="hint-text">Enter sends, Shift+Enter adds a line.</span>
      <span id="voice-status" role="status"></span>
      <button class="btn small" type="button" id="voice-stop" data-action="voice-stop" hidden>Stop reading</button>
      <span class="overseer-slot" id="overseer-slot"></span>
      <span class="autorun-slot" id="autorun-slot"></span>
      <span class="web-slot" id="web-slot"></span>
      <span class="tools-slot" id="tools-slot"></span>
      <span class="effort-slot" id="effort-slot"></span></p>`;
}
