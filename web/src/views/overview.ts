// PROJ-OVERVIEW: the Overview at the top of a project's Tasks pane: what it is, where it lives, what it may
// reach, how far along it is, what happened lately and which models worked on it. Glance first (header,
// four tiles, four lines of About), the rest in sections; the owner edits name, description, about and
// links in place. The data comes from one request (GET /api/projects/{id}/overview).
import { html, type SafeHtml } from "../core/html";
import { relTime } from "../core/time";
import type { Project, ProjectLink, ProjectOverview } from "../api/types";
import { icon } from "./icons";

export type SectionName = "where" | "access" | "build" | "activity" | "ai" | "settings";

export interface OverviewView {
  project: Project; // from the project list: shown while the overview loads
  o: ProjectOverview | undefined; // undefined while loading
  editing: boolean; // the edit form is open
  more: boolean; // About is shown in full
  activityAll: boolean; // Recent activity shows 10, not 5
  preview: boolean; // the edit form shows the About preview
  memberError?: string; // PROJECT-SHARING: why the members row's add form was refused (empty: nothing)
  open: (sec: SectionName) => boolean; // whether a section is expanded
}

const TYPE_LABEL: Record<string, string> = { chat: "Chat", game: "Game", programming: "Programming" };

/** The links field of the edit form: one link per line, "Label https://address". */
export function parseLinks(text: string): ProjectLink[] {
  const lines = text.split("\n");
  const result: ProjectLink[] = [];
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    const parts = trimmed.split(/\s+/);
    const url = parts[parts.length - 1];
    const label = parts.slice(0, -1).join(" ").trim() || url;
    result.push({ label, url });
  }
  return result;
}

/** The links as the edit form shows them, one per line. */
export function linksText(links: ProjectLink[]): string {
  if (links.length === 0) return "";
  return links.map((l) => `${l.label} ${l.url}`).join("\n");
}

/** A grant's rights in words: "Read and write", "Read only", "Run commands", ... */
export function rightsLabel(rights: string[]): string {
  if (rights.length === 0) return "No rights";

  const hasRead = rights.includes("read");
  const hasWrite = rights.includes("write");
  const hasShell = rights.includes("shell");

  let base = "";
  if (hasRead && hasWrite) {
    base = "Read and write";
  } else if (hasRead) {
    base = "Read only";
  } else if (hasWrite) {
    base = "Write";
  }

  if (hasShell) {
    if (base === "") return "Run commands";
    return `${base} and run commands`;
  }

  return base;
}

/** A collapsible section with a header button. `count` is shown next to the title when given. */
export function section(name: SectionName, title: string, count: number | undefined, open: boolean, body: SafeHtml): SafeHtml {
  return html`<section class="ov-sec ${open ? "open" : ""}" data-sec="${name}">
  <button class="ov-sec-head" type="button" data-action="overview-sec" data-sec="${name}" aria-expanded="${open ? "true" : "false"}">${icon(open ? "chevron-down" : "chevron-right")}<span>${title}</span>${count !== undefined ? html`<span class="count">${String(count)}</span>` : ""}</button>
  ${open ? html`<div class="ov-sec-body">${body}</div>` : ""}
</section>`;
}

/** PROJECT-SHARING: only the owner edits, archives and configures a project; members read. */
export function isOwner(v: OverviewView): boolean {
  return (v.o?.role ?? v.project.role ?? "owner") === "owner";
}

function header(v: OverviewView): SafeHtml {
  const name = v.o?.name ?? v.project.name;
  const description = v.o?.description ?? v.project.description;
  const type = v.o?.type ?? v.project.type ?? "chat";
  const kind = v.o?.kind ?? v.project.kind ?? "internal";
  const archivedAt = v.o?.archivedAt ?? v.project.archivedAt;

  return html`
    <header class="ov-head">
      <div class="ov-title">
        <h2 class="ov-name">${name}</h2>
        ${isOwner(v) ? html`<button class="icon-btn" type="button" data-action="overview-edit" data-id="${v.project.id}" aria-label="Edit name, description, about and links">${icon("edit")}</button>` : ""}
        <span class="ov-chips">
          <span class="chip">${TYPE_LABEL[type] ?? type}</span>
          ${kind === "windshift" ? html`<span class="chip">Synced with Windshift</span>` : ""}
          ${archivedAt ? html`<span class="chip archived-chip">Archived</span>` : ""}
        </span>
      </div>
      ${description ? html`<p class="ov-desc">${description}</p>` : html`<p class="ov-desc muted">No description yet.</p>`}
    </header>
      ${archivedAt ? html`<div class="archived-bar" role="status">${icon("archive")}<span>Archived</span>${isOwner(v) ? html`<button class="btn small" data-action="project-unarchive" data-id="${v.project.id}">Unarchive</button>` : ""}</div>` : ""}
  `;
}

function tiles(v: OverviewView): SafeHtml {
  if (!v.o || v.o.progress === null) {
    return html`<div class="ov-tiles loading">
      <div class="ov-tile" data-tile="progress"><span class="ov-num">–</span><span class="ov-label">Done</span><span class="ov-bar" aria-hidden="true"><span style="width: 0%"></span></span></div>
      <div class="ov-tile" data-tile="needs"><span class="ov-num">–</span><span class="ov-label">Needs you</span></div>
      <div class="ov-tile" data-tile="running"><span class="ov-num">–</span><span class="ov-label">Running</span></div>
      <div class="ov-tile" data-tile="activity"><span class="ov-num">–</span><span class="ov-label">Last activity</span></div>
    </div>`;
  }

  const p = v.o.progress;
  const pct = p.total ? Math.round(p.done * 100 / p.total) : 0;
  const last = v.o.activity && v.o.activity.length > 0
    ? relTime(v.o.activity[0].at).replace(/ ago$/, "") // the label already says "Last activity"
    : "None";

  return html`<div class="ov-tiles">
    <div class="ov-tile" data-tile="progress"><span class="ov-num">${String(p.done)}/${String(p.total)}</span><span class="ov-label">Done</span><span class="ov-bar" aria-hidden="true"><span style="width: ${pct}%"></span></span></div>
    <div class="ov-tile ${p.needsYou > 0 ? "attn" : ""}" data-tile="needs"><span class="ov-num">${String(p.needsYou)}</span><span class="ov-label">Needs you</span></div>
    <div class="ov-tile" data-tile="running"><span class="ov-num">${String(p.running)}</span><span class="ov-label">Running</span></div>
    <div class="ov-tile" data-tile="activity"><span class="ov-num">${String(last)}</span><span class="ov-label">Last activity</span></div>
  </div>`;
}

function about(v: OverviewView): SafeHtml {
  const text = v.o?.about ?? "";
  const long = text.length > 280 || text.split("\n").length > 4;

  if (v.o === undefined) {
    return html`<section class="ov-about loading" aria-label="About"><p class="muted">Loading…</p></section>`;
  }

  if (text === "" && !isOwner(v)) {
    return html`<section class="ov-about empty" aria-label="About"><p class="muted">No description of this project yet.</p></section>`;
  }

  if (text === "") {
    return html`<section class="ov-about empty" aria-label="About">
      <p class="muted">Add what this project is for, so anyone sees it at a glance.</p>
      <button class="btn small" type="button" data-action="overview-edit" data-id="${v.project.id}" data-focus="about">${icon("edit")} Edit</button>
    </section>`;
  }

  return html`<section class="ov-about ${long && !v.more ? "clamped" : ""}" aria-label="About">
    <div class="md" data-md-about="${v.project.id}"></div>
    ${long && html`<button class="btn small ov-more" type="button" data-action="overview-more" data-id="${v.project.id}">${v.more ? "Show less" : "Show more"}</button>`}
  </section>`;
}

function where(v: OverviewView): SafeHtml {
  if (!v.o) {
    return html`<p class="muted">Loading…</p>`;
  }
  const w = v.o.where;
  if (w === null) {
    const err = v.o.errors.where ?? "";
    return html`<p class="muted">Could not read this part: ${err}</p>`;
  }

  const m = w.machine;
  const git = w.git
    ? html`<p class="ov-git">${icon("code")}<span>${w.git.remote} · ${w.git.branch}</span></p>`
    : w.gitState === "reading" ? html`<p class="muted">Reading the git remote…</p>`
    : w.gitState === "unknown" ? html`<p class="muted">Git remote unknown while the computer is offline.</p>` : "";
  return html`
    ${m
      ? html`<p class="ov-machine"><span class="dot ${m.online ? "ok" : "off"}" aria-hidden="true"></span>${m.name}${m.os ? ` · ${m.os}` : ""}${m.online ? "" : " · offline"}</p>`
      : html`<p class="muted">No computer yet. Pick one in Settings.</p>`}
    ${w.folder ? html`<p class="ov-folder-row"><code class="ov-folder">${w.folder}</code><button class="icon-btn" type="button" data-action="copy-folder" data-folder="${w.folder}" aria-label="Copy the folder">${icon("copy")}</button></p>` : ""}
    ${git}
    ${v.o.links.length ? html`<p class="ov-links">${v.o.links.map((l) => html`<a class="chip" href="${l.url}" target="_blank" rel="noopener noreferrer">${icon("link")}${l.label}</a>`)}</p>` : ""}`;
}

function until(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, { day: "numeric", month: "short" });
}

export function accessCount(o: ProjectOverview | undefined): number | undefined {
  const a = o?.access;
  if (!a) return undefined;
  return a.grants.length + a.knowledge.length + a.mcp.length + (a.assets > 0 ? 1 : 0);
}

function access(v: OverviewView): SafeHtml {
  if (!v.o) return html`<p class="muted">Loading…</p>`;
  if (v.o.access === null) {
    const err = v.o.errors.access ?? "";
    return html`<p class="muted">Could not read this part: ${err}</p>`;
  }
  const a = v.o.access;
  const items: Array<SafeHtml> = [];

  for (const g of a.grants) {
    items.push(html`<li><span>${rightsLabel(g.rights)}</span> · <code>${g.target}</code>${g.expires ? ` · until ${until(g.expires)}` : ""}</li>`);
  }
  for (const k of a.knowledge) {
    items.push(html`<li><span>Knowledge</span> · ${k.name}</li>`);
  }
  for (const m of a.mcp) {
    items.push(html`<li><span>MCP</span> · ${m.server}${m.tool !== "*" ? ` / ${m.tool}` : ""} · until ${until(m.expires)}</li>`);
  }
  if (a.assets > 0) {
    items.push(html`<li>${String(a.assets)} library assets attached</li>`);
  }

  if (items.length === 0) {
    return html`<p class="muted">Nothing granted yet. Computer grants are set on the Access tab.</p>`;
  }

  return html`<ul class="ov-access">${items}</ul>`;
}

function build(v: OverviewView): SafeHtml {
  if (!v.o) {
    return html`<p class="muted">Loading…</p>`;
  }
  const access = v.o.access;
  if (access === null) {
    return html`<p class="muted">Could not read this part: ${v.o.errors.access ?? ""}</p>`;
  }
  const cmds = access.commands;
  if (cmds.length === 0) {
    return html`<p class="muted">No build, test or run commands yet. Add them in Settings.</p>`;
  }

  const items = cmds.map((c) => {
    const label = c.kind === "build" ? "Build" : c.kind === "test" ? "Test" : c.kind === "run" ? "Run" : "Deploy";
    return html`<li>
      <span class="ov-kind">${label}</span>
      <code>${c.command}</code>
      ${c.last ? html`<span class="dev-result ${c.last.ok ? "ok" : "bad"}">${c.last.ok ? "Passed" : "Failed"} · ${relTime(c.last.at)}</span>` : ""}
      ${c.fromHere ? "" : html`<span class="muted small">never run from here</span>`}
    </li>`;
  });

  return html`<ul class="ov-cmds">${items}</ul>`;
}

function activity(v: OverviewView): SafeHtml {
  if (!v.o) {
    return html`<p class="muted">Loading…</p>`;
  }
  if (v.o.activity === null) {
    const err = v.o.errors.activity ?? "";
    return html`<p class="muted">Could not read this part: ${err}</p>`;
  }
  if (v.o.activity.length === 0) {
    return html`<p class="muted">Nothing yet.</p>`;
  }
  const shown = v.o.activity.slice(0, v.activityAll ? 10 : 5);
  const moreBtn = !v.activityAll && v.o.activity.length > 5
    ? html`<button class="btn small" type="button" data-action="overview-activity-more" data-id="${v.project.id}">More</button>`
    : html``;

  return html`
    <ul class="ov-act">
      ${shown.map((a) => html`
        <li>
          <span class="ov-what">${a.what}</span>
          <span class="ov-when muted">${relTime(a.at)}${a.who ? ` · ${a.who}` : ""}</span>
        </li>
      `)}
    </ul>
    ${moreBtn}
  `;
}

function ai(v: OverviewView): SafeHtml {
  if (!v.o) {
    return html`<p class="muted">Loading…</p>`;
  }
  const aiData = v.o.ai;
  if (aiData === null) {
    const err = v.o.errors.ai ?? "";
    return html`<p class="muted">Could not read this part: ${err}</p>`;
  }
  if (aiData.length === 0) {
    return html`<p class="muted">No model runs in the last 30 days.</p>`;
  }

  const items = aiData.map((m) => {
    const runsText = m.runs === 1 ? "1 run" : `${m.runs} runs`;
    const tokens =
      m.tokens >= 1000
        ? `${(m.tokens / 1000).toFixed(m.tokens >= 100000 ? 0 : 1)}k`
        : String(m.tokens);
    return html`<li><strong>${m.model}</strong> · ${runsText} · ${tokens} tokens · ${String(m.minutes)} min</li>`;
  });

  return html`<ul class="ov-ai">${items}</ul>`;
}

function editForm(v: OverviewView): SafeHtml {
  const name = v.o?.name ?? v.project.name;
  const description = v.o?.description ?? v.project.description;
  const about = v.o?.about ?? "";
  const links = v.o?.links ?? [];
  const synced = (v.o?.kind ?? v.project.kind) === "windshift";
  const linksText = (l: ProjectLink[]) => l.map((x) => `${x.label} ${x.url}`).join("\n");

  return html`<form class="overview-edit" data-id="${v.project.id}">
    <label>Name <input name="name" value="${name}" maxlength="80" required autocomplete="off" ${synced ? "disabled" : ""}></label>
    <label>Description <input name="description" value="${description}" maxlength="200" placeholder="One line: what it is" autocomplete="off" ${synced ? "disabled" : ""}></label>
    ${synced ? html`<p class="muted small">Name and description come from Windshift.</p>` : ""}
    <label>About <textarea name="about" rows="8" placeholder="Markdown: what it is for, who it is for, how it works">${about}</textarea></label>
    <div class="ov-preview-row"><button class="btn small" type="button" data-action="overview-preview" data-id="${v.project.id}">${v.preview ? "Hide preview" : "Preview"}</button></div>
    ${v.preview ? html`<div class="ov-preview md" data-md-preview></div>` : ""}
    <label>Links <textarea name="links" rows="3" placeholder="One per line: Label https://address">${linksText(links)}</textarea></label>
    <div class="ov-edit-actions"><button class="btn small primary" type="submit">Save</button><button class="btn small" type="button" data-action="overview-cancel">Cancel</button></div>
  </form>`;
}

/** PROJECT-SHARING: who has this project: the owner first, then each member with a role; the owner adds and removes people, a member can only leave. */
export function membersRow(v: OverviewView): SafeHtml {
  const o = v.o;
  if (!o) return html``;

  const chips: SafeHtml[] = [
    html`<span class="chip member-chip" data-member="owner">${o.owner} · Owner</span>`,
  ];

  for (const m of o.members) {
    const removable = o.role === "owner" || m.you;
    chips.push(
      html`<span class="chip member-chip" data-member="${m.userId}">
        ${m.name}${m.you ? " (you)" : ""} · ${m.role === "editor" ? "Editor" : "Viewer"}
        ${removable
          ? html`<button class="icon-btn member-remove" type="button" data-action="member-remove" data-id="${v.project.id}" data-user="${m.userId}" aria-label="${m.you ? "Leave this project" : `Remove ${m.name}`}">${icon("close")}</button>`
          : ""}
      </span>`
    );
  }

  let form: SafeHtml = html``;
  if (o.role === "owner") {
    form = html`<form class="member-add" data-id="${v.project.id}">
      <input name="member" placeholder="Add a person" aria-label="Name of the person to add" autocomplete="off" required>
      <select name="role" aria-label="Role"><option value="viewer">Viewer</option><option value="editor">Editor</option></select>
      <button class="btn small" type="submit">Add</button>
      <span class="member-error error" role="alert">${v.memberError ?? ""}</span>
    </form>`;
  }

  return html`<section class="ov-members" aria-label="Members">${chips}${form}</section>`;
}

/** The whole Overview; `settings` is the project's settings panel, shown inside the Settings section. */
export function renderOverview(v: OverviewView, settings: SafeHtml): SafeHtml {
  return html`
    <section class="overview" data-id="${v.project.id}" aria-label="Project overview">
      ${header(v)}
      ${membersRow(v)}
      ${v.editing ? editForm(v) : html`${tiles(v)}${about(v)}`}
      ${section("where", "Where it lives", undefined, v.open("where"), where(v))}
      ${section("access", "Access", accessCount(v.o), v.open("access"), access(v))}
      ${section("build", "Build and test", undefined, v.open("build"), build(v))}
      ${section("activity", "Recent activity", undefined, v.open("activity"), activity(v))}
      ${section("ai", "AI use", undefined, v.open("ai"), ai(v))}
      ${isOwner(v) ? section("settings", "Settings", undefined, v.open("settings"), settings) : ""}
    </section>`;
}
