// KG-01: a project's tasks as a tree (goal > milestone > task > subtask) with dependencies:
// the Plan section of the Tasks pane, the links in a task's details and the editor fields.
import { html, type SafeHtml } from "../core/html";
import type { Task, TaskKind } from "../api/types";
import { stateLabel } from "./tasks";

const KIND_LABELS: Record<TaskKind, string> = { goal: "Goal", milestone: "Milestone", task: "Task" };

/** Whether a project's tasks carry any tree data (otherwise the Plan section stays hidden). */
export function hasTree(tasks: Task[]): boolean {
  return tasks.some((t) => (t.kind && t.kind !== "task") || !!t.parentId || (t.dependsOn?.length ?? 0) > 0);
}

const childrenOf = (tasks: Task[], id: string) =>
  tasks.filter((x) => x.parentId === id).sort((a, b) => (a.position ?? 0) - (b.position ?? 0));

/** Dependencies of `t` that are in `tasks` and not done yet. */
const openDeps = (t: Task, tasks: Task[]) =>
  (t.dependsOn ?? []).filter((d) => tasks.some((x) => x.id === d && x.state !== "done")).length;

function node(t: Task, tasks: Task[], seen: Set<string>): SafeHtml {
  seen.add(t.id);
  const kids = childrenOf(tasks, t.id).filter((c) => !seen.has(c.id));
  const waits = openDeps(t, tasks);
  return html`<li class="tree-node s-${t.state}" data-id="${t.id}">
    <button class="tree-row" data-action="open-task" data-id="${t.id}">
      ${t.kind && t.kind !== "task" ? html`<span class="chip kind">${KIND_LABELS[t.kind]}</span>` : ""}
      <span class="tree-title">${t.title}</span>
      <span class="chip state">${stateLabel(t.state)}</span>
      ${kids.length ? html`<span class="tree-count">${String(kids.filter((c) => c.state === "done").length)}/${String(kids.length)} done</span>` : ""}
      ${waits ? html`<span class="chip waits">Waits on ${String(waits)}</span>` : ""}
    </button>
    ${kids.length ? html`<ul class="tree-children">${kids.map((c) => node(c, tasks, seen))}</ul>` : ""}
  </li>`;
}

/** The Plan section: one project's tasks, roots first (no parent, or a parent outside the list). */
export function renderTree(tasks: Task[]): SafeHtml {
  const seen = new Set<string>();
  const roots = tasks.filter((t) => !t.parentId || !tasks.some((x) => x.id === t.parentId))
    .sort((a, b) => (a.position ?? 0) - (b.position ?? 0));
  return html`<section class="task-tree" aria-label="Plan">
    <h3 class="label">Plan</h3>
    <ul class="tree">${roots.map((r) => node(r, tasks, seen))}</ul>
  </section>`;
}

const link = (x: Task) => html`<button class="link" data-action="open-task" data-id="${x.id}">${x.title}</button>
  <span class="chip state s-${x.state}">${stateLabel(x.state)}</span>`;

/** In a task's details: what it is part of, what it waits for and its subtasks, as links. */
export function treeLinks(t: Task, tasks: Task[]): SafeHtml {
  const parent = tasks.find((x) => x.id === t.parentId);
  const deps = (t.dependsOn ?? []).map((d) => tasks.find((x) => x.id === d)).filter((x): x is Task => !!x);
  const kids = childrenOf(tasks, t.id);
  if (!parent && !deps.length && !kids.length) return html``;
  return html`<dl class="task-links">
    ${parent ? html`<div><dt>Part of</dt><dd>${link(parent)}</dd></div>` : ""}
    ${deps.length ? html`<div><dt>Depends on</dt><dd>${deps.map(link)}</dd></div>` : ""}
    ${kids.length ? html`<div><dt>Subtasks</dt><dd>${kids.map(link)}</dd></div>` : ""}
  </dl>`;
}

/** Editor fields: kind, parent (never itself or one of its subtasks) and dependencies. */
export function treeFields(t: Task | undefined, tasks: Task[]): SafeHtml {
  const kind = t?.kind ?? "task";
  const below = new Set<string>();
  if (t) {
    const stack = [t.id];
    while (stack.length) {
      const id = stack.pop()!;
      if (below.has(id)) continue;
      below.add(id);
      stack.push(...tasks.filter((x) => x.parentId === id).map((x) => x.id));
    }
  }
  const parents = tasks.filter((x) => !below.has(x.id));
  const others = tasks.filter((x) => x.id !== t?.id);
  return html`
    <div class="field">
      <label for="task-kind">Kind</label>
      <select id="task-kind" name="kind">${(Object.keys(KIND_LABELS) as TaskKind[]).map((k) =>
        html`<option value="${k}" ${k === kind ? "selected" : ""}>${KIND_LABELS[k]}</option>`)}</select>
    </div>
    <div class="field">
      <label for="task-parent">Part of</label>
      <select id="task-parent" name="parent" ${kind === "goal" ? "disabled" : ""}>
        <option value="">Nothing (top level)</option>
        ${parents.map((x) => html`<option value="${x.id}" ${x.id === t?.parentId ? "selected" : ""}>${x.title}</option>`)}
      </select>
    </div>
    <fieldset class="field task-deps">
      <legend>Depends on</legend>
      ${others.length ? others.map((x) => html`<label><input type="checkbox" name="deps" value="${x.id}"
        ${t?.dependsOn?.includes(x.id) ? "checked" : ""}> ${x.title}</label>`)
        : html`<p class="muted small">No other tasks in this project yet.</p>`}
    </fieldset>`;
}
