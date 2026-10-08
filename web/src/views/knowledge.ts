// CHAT-03b: knowledge collections on the Capabilities page: a card per collection (title, source,
// size, how far search by meaning is, linked projects), uploads and project links under Details.
import { html, type SafeHtml } from "../core/html";
import { capHead } from "./cap-card";
import { icon } from "./icons";

export interface KDoc { id: number; name: string; chars: number; addedAt: string }
export interface KCollection { id: string; name: string; source: "app" | "openwebui" | "folder"; docs: number; chunks: number; vectors: number; projects: string[]; documents: KDoc[] }
/** A file being uploaded (pct 0-100), or one the server refused (error). */
export interface KUpload { name: string; pct: number; error?: string }

// Open Details, by collection id, so the live refresh keeps them open.
const openKeys = new Set<string>();
document.addEventListener("toggle", (e) => {
  const d = e.target;
  if (d instanceof HTMLDetailsElement && d.dataset.keep?.startsWith("kn|")) {
    if (d.open) openKeys.add(d.dataset.keep);
    else openKeys.delete(d.dataset.keep);
  }
}, true);

export function renderKnowledge(cols: KCollection[] | undefined, projects: { id: string; name: string }[], uploads: Record<string, KUpload[]>): SafeHtml {
  // The page as it is now wins: a Details opened just before this render has not sent its toggle yet.
  document.querySelectorAll<HTMLDetailsElement>('details[data-keep^="kn|"]').forEach((d) => {
    if (d.open) openKeys.add(d.dataset.keep ?? "");
    else openKeys.delete(d.dataset.keep ?? "");
  });
  const size = (chars: number) => chars >= 1000 ? `${Math.round(chars / 1000)}k characters` : `${chars} characters`;
  const meaning = (c: KCollection) => c.chunks === 0 ? "" : c.vectors >= c.chunks ? " · search by meaning ready" : ` · search by meaning ${Math.floor((c.vectors / c.chunks) * 100)}%`;
  const projectName = (id: string) => projects.find((p) => p.id === id)?.name;

  const card = (c: KCollection) => {
    const names = c.projects.map(projectName).filter((n): n is string => !!n);
    const ups = uploads[c.id] ?? [];
    return html`<li class="task cap kn-card">
    <div class="task-main">
      ${capHead("folder", c.name, c.source === "openwebui" ? "Open WebUI" : c.source === "folder" ? "folder" : "uploaded")}
      <span class="task-step">${c.docs} documents · ${c.chunks} parts${meaning(c)}</span>
      ${names.length > 0 ? html`<span class="kn-projects">${names.map((n) => html`<span class="chip">${n}</span>`)}</span>` : ""}
      ${ups.length > 0 ? html`<ul class="kn-uploads">${ups.map((u) => html`<li class="kn-up ${u.error ? "failed" : ""}"><span class="kn-up-name">${u.name}</span>${u.error ? html`<span class="kn-err" role="alert">${u.error}</span>` : html`<progress max="100" value="${u.pct}">${u.pct}%</progress>`}</li>`)}</ul>` : ""}
      <details class="kn-more" data-keep="kn|${c.id}" ${openKeys.has(`kn|${c.id}`) ? "open" : ""}>
        <summary>Details</summary>
        ${c.docs > 0 ? html`<button type="button" class="btn small" data-action="kn-browse" data-id="${c.id}" data-name="${c.name}">${icon("folder")} Read documents</button>` : ""}
        ${c.source === "app"
          ? html`<label class="btn small kn-add">${icon("plus")} Add files<input type="file" class="kn-file sr-only" data-id="${c.id}" multiple accept=".md,.markdown,.txt,.text,.html,.htm,.pdf,.rst,.csv,.json"></label>`
          : html`<p class="muted small kn-synced">${c.source === "folder" ? "Kept in step with a folder on the server: add files there." : "Kept in step with Open WebUI: add files there."}</p>`
        }
        <fieldset class="kn-link"><legend>Use in projects</legend>
          ${projects.length ? projects.map((p) => html`<label class="kn-check"><input type="checkbox" class="kn-project" data-id="${c.id}" value="${p.id}" ${c.projects.includes(p.id) ? "checked" : ""}> ${p.name}</label>`) : html`<p class="muted small">No projects yet.</p>`}
        </fieldset>
        <ul class="kn-docs">${c.documents.map((d) => html`<li><span class="kn-doc-name" title="${d.name}">${d.name}</span><span class="muted small">${size(d.chars)}</span>${c.source === "app" ? html`<button type="button" class="icon-btn" data-action="kn-doc-delete" data-id="${c.id}" data-doc="${String(d.id)}" aria-label="Remove ${d.name}">${icon("trash")}</button>` : ""}</li>`)}</ul>
        ${c.docs > c.documents.length ? html`<p class="muted small">and ${c.docs - c.documents.length} more</p>` : ""}
        <button type="button" class="btn small danger" data-action="kn-delete" data-id="${c.id}" data-name="${c.name}">${icon("trash")} Delete collection</button>
      </details>
    </div>
  </li>`;
  };

  return html`<section class="caps-group knowledge" aria-labelledby="caps-knowledge">
    <h2 id="caps-knowledge">Knowledge <span class="muted">${cols?.length ?? 0}</span></h2>
    <form class="kn-new"><label class="sr-only" for="kn-name">New collection</label><input id="kn-name" name="name" placeholder="New collection" maxlength="80" required><button class="btn small" type="submit">${icon("plus")} Create</button></form>
    ${cols === undefined ? html`<p class="muted small">Loading collections…</p>`
      : cols.length === 0 ? html`<p class="muted small">No collections yet. Create one and add Markdown, text, HTML or PDF files.</p>`
      : html`<ul class="caps-cards">${cols.map(card)}</ul>`}
  </section>`;
}
