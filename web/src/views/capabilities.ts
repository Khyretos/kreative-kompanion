// Capabilities: what Kompanion can use right now.
import { html, type SafeHtml } from "../core/html";
import { renderTimeline, type TlGpu } from "./gputimeline";
import { relTime } from "../core/time";
import { icon } from "./icons";
import { pageHead } from "./pagehead";
import { capHead, capStep } from "./cap-card";

export interface CapModel {
  id: string;
  name: string;
  local: boolean;
  status: "ok" | "down";
  error: string | null;
  models: string[];
  roles: string[];
  lastError: { text: string; at: string } | null;
}

export interface CapGrant {
  target: string;
  rights: string[];
  expires: string | null;
}

export interface CapComputer {
  id: string;
  name: string;
  online: boolean;
  lastSeen: string;
  runnerVersion?: string | null;
  grants: CapGrant[];
}

export interface CapTool {
  name: string;
  description: string;
  needs: string;
}

export interface CapIndex {
  id: string;
  name: string;
  items: number;
  of: number;
  failed: number;
  model: string;
  status: "ok" | "partly" | "empty";
}

export interface CapSkill {
  id: string;
  title: string;
  lessons: number;
  updated: string;
  /** SK-03: general (kompas-skills), kompanion (this repo) or private (this setup). */
  layer: "general" | "kompanion" | "private";
  /** The path inside the layer, e.g. "shared/git.md". */
  file: string;
}

export interface SkillCommit { sha: string; message: string; author: string; date: string }

export interface CapHolding { name: string; kind: string; nowMib: number; peakMib: number; busy: boolean }
export interface CapGpu { id: string; machine: string; totalMib: number; usedMib: number | null; reservedMib: number; otherMib: number; freeMib: number; schedulable: boolean; holdings: CapHolding[] }
export interface CapSwitch { from: string; to: string; by: string; startedAt: string; seconds: number; ok: boolean; coderAnswerS: number | null; error: string | null }
/** GPU-01: what a computer's GPU is for; "effective" is Gaming while a game runs (or 10 min after) in Auto. */
export type GpuModeName = "studio" | "gaming" | "auto";
const gpuLabels: [GpuModeName, string][] = [
  ["studio", "Studio on"],
  ["gaming", "Studio off"],
  ["auto", "Auto"],
];

/** GPU-01: the mode switch and one line saying what the mode does right now. */
function gpuModeSwitch(m: CapGpuMode): SafeHtml {
  const text =
    !m.granted
      ? `Grant "GPU apps" for ${m.machine} under Machines, Access, so Kompanion can stop and start its studio apps.`
      : m.effective === "gaming" && m.gaming
        ? "Auto turned the studio off: a game is running. Studio apps stopped, Ollama unloaded."
        : m.effective === "gaming" && m.mode === "auto"
          ? "Auto keeps the studio off for 10 min after the game ended."
          : m.effective === "gaming"
            ? "Studio off: the studio apps are stopped and Ollama is unloaded. Studio jobs go elsewhere or wait."
            : m.appsStopped
              ? `Studio apps stopped; the next studio job starts them on ${m.machine}.`
              : m.mode === "studio"
                ? "Studio on: the apps may run; nothing is stopped."
                : "Auto: the studio apps stop after 15 min without a studio job, and while a game runs.";

  return html`
    <span class="seg gpu-mode" role="group" aria-label="GPU mode on ${m.machine}">
      ${gpuLabels.map(([k, label]) =>
        html`<button type="button" data-action="gpu-mode" data-machine="${m.machine}" data-mode="${k}" aria-pressed="${m.mode === k ? "true" : "false"}">${label}</button>`
      )}
    </span>
    <span class="task-meta gpu-mode-state">${text}</span>
  `;
}

export interface CapGpuMode { machine: string; mode: GpuModeName; effective: GpuModeName; gaming: boolean; appsStopped: boolean; granted: boolean; studioAt: string | null; gpus: string[]; apps: string[] }
/** STU-01 (M6-05 step 4): a saved workflow with its licences, GPUs and run numbers. */
export interface CapWorkflow { name: string; title: string; description: string; studio?: { label: string } | null; base?: string | null;
  models: { file: string; licence: string }[]; problems: string[]; runnable: boolean; targets: string[]; runs: number; lastRun: string | null; avgSeconds: number | null }
/** GPU-03: where the studio runs ("auto", a GPU id or "off"), the choices and their cost. */
export interface CapStudioChoice { value: string; label: string; cost: string }
export interface CapStudioTarget { target: string; choices: CapStudioChoice[]; queued: number }
export interface CapRole { gpu: string | null; mode: string; app: string | null; switching: string | null; last: CapSwitch | null }

export interface Capabilities {
  gpus?: CapGpu[];
  gpuRole?: CapRole | null;
  gpuModes?: CapGpuMode[];
  studioTarget?: CapStudioTarget | null;
  workflows?: CapWorkflow[];
  models: CapModel[];
  computers: CapComputer[];
  tools: CapTool[];
  mcp: CapMcp[];
  indexes: CapIndex[];
  skills: CapSkill[];
}

function modelCard(m: CapModel): SafeHtml {
  const state = m.status === "ok" ? "done" : "failed";
  const label = m.status === "ok" ? "online" : "down";
  return html`
    <li class="task cap s-${state}">
      <div class="task-main">
        ${capHead("spark", m.name, label, m.local ? "local" : "cloud")}
        ${capStep(m.status === "down" ? (m.error ?? "") : `${m.models.length} model(s): ${m.models.slice(0, 4).join(", ")}${m.models.length > 4 ? "…" : ""}`)}
        <span class="task-meta">
          ${icon("spark")} ${m.roles.length ? m.roles.join(" · ") : "no role uses it"}
          ${m.lastError ? html` · last error <span class="cap-err" title="${m.lastError.text}">${relTime(m.lastError.at)}</span>` : ""}
        </span>
      </div>
    </li>
  `;
}

function computerCard(c: CapComputer): SafeHtml {
  const state = c.online ? "done" : "queued";
  const label = c.online ? "online" : "offline";
  const grantsText = c.grants.length
    ? c.grants.map((g) => `${g.target} (${g.rights.join(", ")})`).join(" · ")
    : "No grants: every step asks first";
  return html`
    <li class="task cap s-${state}">
      <div class="task-main">
        ${capHead("pc", c.name, label)}
        ${capStep(grantsText)}
        <span class="task-meta">
          ${icon("pc")} ${c.online ? "seen " : "last seen "}
          ${c.lastSeen ? relTime(c.lastSeen) : "never"} · runner ${c.runnerVersion ?? "before 0.4.5"}
        </span>
      </div>
    </li>
  `;
}

function toolCard(t: CapTool): SafeHtml {
  return html`
    <li class="task cap s-done">
      <div class="task-main">
        ${capHead("terminal", html`<code>${t.name}</code>`, "tool")}
        ${capStep(t.description)}
        <span class="task-meta">needs: ${t.needs || "nothing"}</span>
      </div>
    </li>
  `;
}

/** CHAT-01: an MCP tool server from [[mcp]] in kompanion.toml: ok with its tools, down, or off. */
export interface CapMcp { name: string; status: "ok" | "down" | "off"; description?: string | null; error?: string; tools: { name: string; description?: string | null }[] }

function mcpCard(m: CapMcp): SafeHtml {
  const state = m.status === "ok" ? "done" : m.status === "down" ? "failed" : "queued";
  return html`
    <li class="task cap s-${state}">
      <div class="task-main">
        ${capHead("link", m.name, m.status, m.description ?? undefined)}
        ${capStep(m.status === "down" ? (m.error ?? "not reachable") : m.tools.length ? `${m.tools.length} tools: ${m.tools.map((t) => t.name).join(", ")}` : "No tools")}
      </div>
    </li>
  `;
}

function indexCard(i: CapIndex): SafeHtml {
  const state = i.status === "ok" ? "done" : i.status === "partly" ? "needs_input" : "queued";
  const label = i.status === "ok" ? "ready" : i.status === "partly" ? "partly" : "empty";
  const step = `${i.items.toLocaleString("en")} of ${i.of.toLocaleString("en")} items${i.failed ? ` · ${i.failed} failed` : ""}`;
  return html`
    <li class="task cap s-${state}">
      <div class="task-main">
        ${capHead("search", i.name, label)}
        ${capStep(step)}
        <span class="task-meta">${icon("spark")} ${i.model}</span>
      </div>
    </li>
  `;
}

function skillCard(s: CapSkill): SafeHtml {
  return html`
    <li class="task cap s-done">
      <button class="task-main" data-action="open-skill" data-id="${s.id}" data-layer="${s.layer}" data-file="${s.file}">
        ${capHead("braces", s.title, `${s.lessons} lessons`, `${s.layer} layer`)}
        <span class="task-step"><code>${s.id}</code></span>
        <span class="task-meta">${s.updated ? `updated ${relTime(s.updated)}` : ""}</span>
      </button>
    </li>
  `;
}

const gb = (mib: number) => `${(mib / 1024).toFixed(1)} GB`;


/** GPU-03: "Studio runs on": one button per choice, and what the current choice means. */
function studioTargetControl(t: CapStudioTarget): SafeHtml {
  const chosen = t.choices.find((c) => c.value === t.target);
  let text = "";
  if (chosen) {
    text = chosen.cost || `Studio jobs run on ${chosen.label}.`;
  }

  const queuedText = t.queued > 0 ? ` · ${t.queued} studio ${t.queued === 1 ? "job" : "jobs"} queued` : "";

  return html`<section class="caps-group studio-target" aria-labelledby="caps-studio-target">
    <h2 id="caps-studio-target">Studio runs on</h2>
    <span class="seg" role="group" aria-label="Studio runs on">${t.choices.map((c) =>
      html`<button type="button" data-action="studio-target" data-target="${c.value}" aria-pressed="${t.target === c.value ? "true" : "false"}" title="${c.cost}">${c.label}</button>`)}</span>
    <p class="muted small studio-target-cost">${text}${queuedText}</p>
  </section>`;
}

function workflowCard(w: CapWorkflow): SafeHtml {
  const licences = [...new Set(w.models.map((m) => m.licence))].join(", ");
  const kind = w.studio && w.base ? `Image type, based on ${w.base}` : w.studio ? "Image type" : w.base ? `Based on ${w.base}` : "";
  const runs = w.runs === 0 ? "No runs yet" : `${w.runs} ${w.runs === 1 ? "run" : "runs"}${w.avgSeconds !== null ? ` · ${Math.round(w.avgSeconds)} s on average` : ""}${w.lastRun ? ` · last ${relTime(w.lastRun)}` : ""}`;

  return html`<li class="task cap s-${w.problems.length ? "needs_input" : "done"}">
    <div class="task-main">
      ${capHead("image", w.title, w.problems.length ? "licence warning" : "ready", kind || undefined)}
      ${capStep(w.description)}
      <span class="task-meta">${licences} · on ${w.targets.length ? w.targets.join(", ") : "no GPU"}</span>
      <span class="task-meta">${runs}</span>
      ${w.problems.length ? html`<span class="task-meta wf-problems">licence warning: ${w.problems.join("; ")}</span>` : ""}
    </div>
  </li>`;
}

/** M6-01: what each GPU holds (at its peak: weights plus KV cache), what else uses it, what is free. */
function gpuCard(g: CapGpu, role?: CapRole | null, modes?: CapGpuMode[]): SafeHtml {
  const gm = modes?.find((m) => m.gpus.includes(g.id));
  const r = role && role.gpu === g.id ? role : null;
  const roleText = r ? (r.switching ? `switching to ${r.switching}…` : r.mode === "artist" && r.app ? `artist (${r.app})` : r.mode) : "";
  const last = r?.last ? `last switch ${r.last.startedAt.slice(11, 16)}: ${r.last.from} → ${r.last.to}, ${Math.round(r.last.seconds)} s${r.last.coderAnswerS !== null ? `, Coder answered in ${Math.round(r.last.coderAnswerS)} s` : ""}${r.last.ok ? "" : ` (failed: ${r.last.error ?? "unknown"})`}` : "";
  const state = !g.schedulable ? "queued" : g.freeMib < 1024 ? "needs_input" : "done";
  const label = !g.schedulable ? "protected" : `${gb(g.freeMib)} free`;
  const pct = (mib: number) => `${Math.min(100, Math.round((mib / Math.max(1, g.totalMib)) * 100))}%`;
  return html`
    <li class="task cap s-${state}">
      <div class="task-main">
        ${capHead("box", `${g.id} · ${gb(g.totalMib)}`, label, r ? html`${g.machine} <span class="chip role" title="Coder (OVMS) or one studio app; switches by itself (M6-03)">${roleText}</span>` : g.machine)}
        <span class="vram-bar" role="img" aria-label="${`${gb(g.reservedMib)} reserved, ${gb(g.otherMib)} other, ${gb(g.freeMib)} free`}">
          <span class="vb-res" style="width:${pct(g.reservedMib)}"></span><span class="vb-other" style="width:${pct(g.otherMib)}"></span>
        </span>
        ${capStep(g.holdings.length
          ? g.holdings.map((h) => `${h.name} ${gb(Math.max(h.nowMib, h.peakMib))}${h.busy ? " (busy)" : ""}`).join(" · ")
          : "Nothing loaded")}
        ${gm ? gpuModeSwitch(gm) : ""}
        <span class="task-meta">${icon("spark")} reserved ${gb(g.reservedMib)} · other ${gb(g.otherMib)}${g.usedMib === null ? "" : ` · measured ${gb(g.usedMib)}`}</span>
        ${last ? html`<span class="task-meta">${last}</span>` : ""}
      </div>
    </li>`;
}

function group(key: string, title: string, items: SafeHtml[], empty: string): SafeHtml {
  return html`
    <section class="caps-group" aria-labelledby="caps-${key}">
      <h2 id="caps-${key}">${title} <span class="muted">${items.length}</span></h2>
      ${items.length ? html`<ul class="caps-cards">${items}</ul>` : html`<p class="muted small">${empty}</p>`}
    </section>
  `;
}

export function renderCapabilities(c: Capabilities | undefined, timeline?: TlGpu[], range: 1 | 24 = 1, gpus = true, knowledge: SafeHtml | string = ""): SafeHtml {
  if (c === undefined) {
    return html`
      <div class="caps">
        ${pageHead("Capabilities", "What Kompanion can use right now. Updates live.")}
        <p class="muted caps-loading">Checking models and computers…</p>
      </div>
    `;
  }

  const models = c.models.map((x) => modelCard(x));
  const computers = c.computers.map((x) => computerCard(x));
  const tools = c.tools.map((x) => toolCard(x));
  const mcp = c.mcp.map((x) => mcpCard(x));
  const indexes = c.indexes.map((x) => indexCard(x));
  const skills = c.skills.map((x) => skillCard(x));

  return html`
    <div class="caps">
      ${pageHead("Capabilities", "What Kompanion can use right now. Updates live.")}
      ${gpus && c.studioTarget ? studioTargetControl(c.studioTarget) : ""}
      ${gpus ? group("gpus", "GPUs", (c.gpus ?? []).map((g) => gpuCard(g, c.gpuRole, c.gpuModes)), "No GPUs configured (kompanion.toml [[gpu]]).") : ""}
      ${gpus ? group("workflows", "Workflows", (c.workflows ?? []).map((w) => workflowCard(w)), "No saved workflows yet (studio/workflows).") : ""}
      ${gpus && (c.gpus ?? []).length ? renderTimeline(timeline, range, Object.fromEntries((c.gpus ?? []).map((g) => [g.id, g.totalMib]))) : ""}
      ${group("models", "Models", models, "No model providers are configured.")}
      ${group("computers", "Computers", computers, "No computer is paired yet.")}
      ${group("tools", "Tools", tools, "No tools found.")}
      ${group("mcp", "MCP servers", mcp, "No MCP servers yet.")}
      ${knowledge}
      ${group("indexes", "Search indexes", indexes, "No indexes yet.")}
      ${group("skills", "Skills", skills, "No skills found.")}
    </div>
  `;
}
