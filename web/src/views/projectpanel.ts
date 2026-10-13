// Project settings at the top of the Tasks pane: the type, and per type its extra part
// (programming: repo folder and computer; game: attached library assets and a picker).
import { html, type SafeHtml } from "../core/html";
import type { DevSettingsView, Project } from "../api/types";
import { icon } from "./icons";

export type ProjectType = "chat" | "game" | "programming";

export interface ProjectAsset {
  id: number;
  name: string;
  category: string;
  pack: string;
  preview: "image" | "audio" | null;
  pv: number;
  missing: boolean;
}

export interface PickResult {
  id: number;
  name: string;
  category: string;
  pack: string;
  preview: "image" | "audio" | null;
  pv: number;
}

export interface PanelOptions {
  dev?: DevSettingsView; // KG-10: build/test/run/deploy settings, undefined while loading
  machines: { id: string; name: string }[];
  assets: ProjectAsset[] | undefined;
  pick: { q: string; items: PickResult[]; busy: boolean };
  thumb: (a: { id: number; pv: number }) => string;
}

const TYPES: [ProjectType, string][] = [["chat", "Chat"], ["game", "Game (attach assets)"], ["programming", "Programming (repo folder)"]];

function assetCard(a: ProjectAsset, p: Project, o: PanelOptions): SafeHtml {
  return html`<li class="task project-asset ${a.missing ? "s-failed" : "s-done"}">
    <div class="task-main">
      ${a.preview === "image"
        ? html`<img class="pa-thumb" src="${o.thumb(a)}" alt="" loading="lazy">`
        : html`<span class="pa-thumb pa-icon" aria-hidden="true">${icon(a.preview === "audio" ? "spark" : "box")}</span>`}
      <span class="task-title">${a.name}</span>
      <span class="task-meta">${a.category} · ${a.pack}${a.missing ? " · missing from the library" : ""}</span>
    </div>
    <button class="icon-btn pa-remove" data-action="detach-asset" data-project="${p.id}" data-id="${String(a.id)}" aria-label="Detach ${a.name}">${icon("close")}</button>
  </li>`;
}

function pickRow(r: PickResult, p: Project, o: PanelOptions): SafeHtml {
  const isAttached = o.assets?.some((x) => x.id === r.id);
  return html`<li>
    <span class="pick-name">${r.name}</span>
    <span class="muted small">${r.category} · ${r.pack}</span>
    ${isAttached ? html`<button class="btn small" disabled>Attached</button>` : html`<button class="btn small" data-action="attach-asset" data-project="${p.id}" data-id="${String(r.id)}">Attach</button>`}
  </li>`;
}

function repoForm(p: Project, o: PanelOptions): SafeHtml {
  if (o.machines.length === 0) return html`<p class="muted small">Pair a computer first (Machines tab).</p>`;
  return html`
    <form class="project-repo" data-id="${p.id}">
      <label>Computer <select name="machine">
        <option value="">Pick a computer</option>
        ${o.machines.map((m) => html`<option value="${m.id}" ${m.id === p.repoMachineId ? "selected" : ""}>${m.name}</option>`)}
      </select></label>
      <label>Folder <input name="folder" value="${p.repoFolder ?? ""}" placeholder="/home/you/projects/app"></label>
      <button class="btn small primary" type="submit">Save</button>
      <p class="muted small">Used as the computer and folder when you run a task of this project.</p>
    </form>`;
}

function devForm(p: Project, d: DevSettingsView | undefined): SafeHtml {
  if (!d) return html`<p class="muted small">Loading…</p>`;
  const s = d.settings;
  const field = (name: "engine" | "build" | "test" | "run" | "deploy" | "screenshots", label: string, hint: string): SafeHtml =>
    html`<label>${label} <input name="${name}" value="${s[name]}" placeholder="${hint}" autocomplete="off" spellcheck="false"></label>`;
  const result = (kind: "build" | "test" | "run"): SafeHtml => {
    const r = s.last[kind];
    return r ? html`<span class="dev-result ${r.ok ? "ok" : "bad"}" data-kind="${kind}">${r.ok ? "Passed" : "Failed"} · ${r.summary.split("\n").pop() ?? ""}</span>` : html``;
  };
  const runnable = !!d.repoMachineId && !!d.repoFolder;
  const runBtn = (kind: "build" | "test" | "run"): SafeHtml =>
    html`<span class="dev-run"><button class="btn small" type="button" data-action="dev-run" data-project="${p.id}" data-kind="${kind}" ${runnable && s[kind].trim() ? "" : "disabled"}>${kind === "build" ? "Build" : kind === "test" ? "Test" : "Run"} now</button>${result(kind)}</span>`;
  return html`
    <form class="project-dev" data-id="${p.id}">
      <h3 class="label">Build and test</h3>
      <label>Start from <select name="preset" id="dev-preset">
        <option value="">None</option>
        ${d.presets.map((x) => html`<option value="${x.preset}" ${x.preset === s.preset ? "selected" : ""}>${x.preset === "game" ? "Game (kk-engine)" : "Software"}</option>`)}
      </select></label>
      ${field("engine", "Engine", "kk-engine")}
      ${field("build", "Build", "make")}
      ${field("test", "Test", "make test")}
      ${field("run", "Run", "./app --headless")}
      ${field("deploy", "Deploy", "sh deploy.sh")}
      ${field("screenshots", "Screenshot folder", "screenshots")}
      <div class="dev-actions">
        <button class="btn small primary" type="submit">Save</button>
        ${runBtn("build")} ${runBtn("test")} ${runBtn("run")}
      </div>
      <p class="muted small">${runnable ? "Commands run in the repo folder under this project's grants. The test command also checks scheduled tasks." : "Pick the computer and folder above to run them."}</p>
    </form>`;
}

function gameAssets(p: Project, o: PanelOptions): SafeHtml {
  const list = o.assets === undefined
    ? html`<p class="muted small">Loading…</p>`
    : o.assets.length === 0
      ? html`<p class="muted small">No assets yet. Find them below and attach them.</p>`
      : html`<ul class="project-assets">${o.assets.map((a) => assetCard(a, p, o))}</ul>`;
  return html`
    <h3 class="label">Assets <span class="count">${o.assets?.length ?? 0}</span></h3>
    ${list}
    <div class="asset-picker">
      <label class="sr-only" for="asset-pick-q">Find assets to attach</label>
      <input type="search" id="asset-pick-q" data-project="${p.id}" value="${o.pick.q}" placeholder="Find assets to attach (name, pack, category)" autocomplete="off">
      ${o.pick.busy ? html`<p class="muted small" role="status">Searching…</p>` : ""}
      ${o.pick.q && !o.pick.busy && o.pick.items.length === 0 ? html`<p class="muted small" role="status">Nothing found.</p>` : ""}
      <ul class="pick-results">${o.pick.items.map((r) => pickRow(r, p, o))}</ul>
    </div>`;
}

export function renderProjectPanel(p: Project, o: PanelOptions): SafeHtml {
  const type = p.type ?? "chat";
  return html`
    <section class="project-panel" aria-label="Project settings">
      <div class="project-type">
        <label for="project-type">Project type</label>
        <select id="project-type" data-id="${p.id}">
          ${TYPES.map(([value, label]) => html`<option value="${value}" ${value === type ? "selected" : ""}>${label}</option>`)}
        </select>
      </div>
      <label class="project-overseer" title="Every chat in this project is answered by the Overseer, which sees all your projects and tasks. The project thread stays as it is.">
        <input type="checkbox" id="project-overseer" data-id="${p.id}" ${p.overseer ? "checked" : ""}>${icon("eye")}<span>Overseer always on in this project</span>
      </label>
      ${type === "programming" || type === "game" ? repoForm(p, o) : ""}
      ${type === "game" ? gameAssets(p, o) : ""}
      ${type === "programming" || type === "game" ? devForm(p, o.dev) : ""}
    </section>`;
}
