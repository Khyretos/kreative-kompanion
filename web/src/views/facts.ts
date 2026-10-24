// FACTS: the Facts tab: what Kompanion remembers for this chat's project and for this chat.
// Add, edit in place and delete (with Undo); search over names and values; flagged facts first.
import { html, type SafeHtml } from "../core/html";
import { relTime } from "../core/time";
import type { Fact, FactKind, FactsView } from "../api/types";
import { icon } from "./icons";

export const KINDS: { id: FactKind; label: string }[] = [
  { id: "text", label: "Text" },
  { id: "path", label: "Path" },
  { id: "url", label: "Link" },
  { id: "location", label: "Place" },
  { id: "other", label: "Other" },
];

export interface FactsUi {
  view?: FactsView;
  projectName?: string;
  query: string;
  editingId?: string;
  hidden: string[]; // deleted, waiting for Undo
}

function matches(f: Fact, q: string): boolean {
  if (!q) return true;
  const hay = `${f.key}\n${f.value}`.toLowerCase();
  return q.toLowerCase().split(/\s+/).filter(Boolean).every((w) => hay.includes(w));
}

function kindSelect(name: string, current: FactKind, id: string): SafeHtml {
  return html`<select name="${name}" id="${id}" aria-label="Kind">${KINDS.map((k) => html`<option value="${k.id}" ${k.id === current ? "selected" : ""}>${k.label}</option>`)}</select>`;
}

function valueView(f: Fact): SafeHtml {
  if (f.kind === "url" && /^https?:\/\//i.test(f.value)) return html`<a class="fact-value" href="${f.value}" target="_blank" rel="noopener noreferrer">${f.value}</a>`;
  return html`<span class="fact-value${f.kind === "path" || f.kind === "other" ? " mono" : ""}">${f.value}</span>`;
}

function row(f: Fact, canEdit: boolean, editing: boolean): SafeHtml {
  if (editing) {
    return html`<li class="fact editing"><form class="fact-edit" data-id="${f.id}">
      <label class="sr-only" for="fact-key-${f.id}">Name</label>
      <input name="key" id="fact-key-${f.id}" value="${f.key}" maxlength="80" required>
      <label class="sr-only" for="fact-value-${f.id}">Value</label>
      <textarea name="value" id="fact-value-${f.id}" rows="2" maxlength="2000" required>${f.value}</textarea>
      <div class="fact-actions">${kindSelect("kind", f.kind, `fact-kind-${f.id}`)}
        <button class="btn small primary" type="submit">Save</button>
        <button class="btn small" type="button" data-action="fact-cancel">Cancel</button></div>
    </form></li>`;
  }
  const who = f.source === "user" ? "you" : "Kompanion";
  const kind = KINDS.find((k) => k.id === f.kind)?.label ?? f.kind;
  return html`<li class="fact${f.flaggedAt ? " flagged" : ""}" data-id="${f.id}">
    <div class="fact-main">
      <span class="fact-key">${f.key}</span>
      ${valueView(f)}
      <span class="fact-meta">${f.flaggedAt ? html`<span class="chip fact-flag" title="A step found this path missing: check it and edit it">${icon("alert")} May be out of date</span>` : ""}
        <span class="chip fact-kind">${kind}</span> Set by ${who} · ${relTime(f.updatedAt)}</span>
    </div>
    ${canEdit ? html`<div class="fact-buttons">
      <button class="icon-btn" type="button" data-action="fact-edit" data-id="${f.id}" aria-label="Edit ${f.key}" title="Edit">${icon("edit")}</button>
      <button class="icon-btn" type="button" data-action="fact-delete" data-id="${f.id}" aria-label="Delete ${f.key}" title="Delete">${icon("trash")}</button>
    </div>` : ""}
  </li>`;
}

function group(title: string, scope: "project" | "chat", list: Fact[] | undefined, canEdit: boolean, ui: FactsUi, hint: string): SafeHtml {
  if (!list) return html``;
  const shown = list.filter((f) => !ui.hidden.includes(f.id) && matches(f, ui.query));
  const total = list.filter((f) => !ui.hidden.includes(f.id)).length;
  const sid = `${scope}`;
  return html`<section class="facts-group" data-scope="${scope}" aria-labelledby="facts-h-${sid}">
    <h3 id="facts-h-${sid}">${title} <span class="count">${total}</span></h3>
    <p class="facts-hint">${hint}</p>
    ${canEdit ? html`<form class="fact-add" data-scope="${scope}">
      <label class="sr-only" for="fact-add-key-${sid}">Name</label>
      <input name="key" id="fact-add-key-${sid}" placeholder="Name, e.g. Deploy command" maxlength="80" required>
      <label class="sr-only" for="fact-add-value-${sid}">Value</label>
      <input name="value" id="fact-add-value-${sid}" placeholder="Value: a note, path, link, place…" maxlength="2000" required>
      <div class="fact-actions">${kindSelect("kind", "text", `fact-add-kind-${sid}`)}<button class="btn small primary" type="submit">${icon("plus")} Add</button></div>
    </form>` : html`<p class="facts-hint">You can read these facts; the owner or an editor changes them.</p>`}
    ${shown.length ? html`<ul class="facts-list">${shown.map((f) => row(f, canEdit, ui.editingId === f.id))}</ul>`
      : html`<p class="facts-empty">${ui.query && total ? "No fact matches your search." : "No facts yet."}</p>`}
  </section>`;
}

export function renderFacts(ui: FactsUi): SafeHtml {
  const v = ui.view;
  if (!v) return html`<p class="facts-empty">Open a chat or pick a project to see what Kompanion remembers for it.</p>`;
  return html`<div class="facts-pane">
    <p class="facts-intro">Kompanion reads these in every answer and saves its own. Never put passwords or tokens here: write where they live.</p>
    <label class="sr-only" for="facts-q">Search facts</label>
    <input type="search" id="facts-q" class="facts-search" placeholder="Search facts" value="${ui.query}">
    ${v.project ? group(ui.projectName ? `Project · ${ui.projectName}` : "Project", "project", v.project, !!v.canEditProject, ui, "Every chat in this project sees these.") : ""}
    ${v.chat ? group("This chat", "chat", v.chat, !!v.canEditChat, ui, "Only this chat sees these.") : ""}
  </div>`;
}
