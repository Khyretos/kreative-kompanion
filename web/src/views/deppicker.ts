// TASK-FORM: picker for task dependencies in the editor

import { html, type SafeHtml } from "../core/html";
import type { Task } from "../api/types";
import { icon } from "./icons";

/**
 * Renders the "Depends on" field of the task editor as chips plus a search box plus a list.
 * Every option task is rendered twice: once as a chip (shown when its id is in selected) and
 * once as a list option (shown when it is not), so later code only toggles attributes.
 */
export function depPicker(selected: string[], options: Task[]): SafeHtml {
  if (options.length === 0) {
    return html`<div class="field"><span class="label">Depends on</span>
      <p class="muted small">No other tasks in this project yet.</p></div>`;
  }

  const on = (id: string) => selected.includes(id);

  return html`<div class="field dep-picker">
    <label for="dep-search">Depends on</label>
    <ul class="dep-chips" aria-label="Chosen tasks">${options.map((x) => html`
      <li class="dep-chip" data-id="${x.id}" ${on(x.id) ? "" : "hidden"}>
        <input type="hidden" name="deps" value="${x.id}" ${on(x.id) ? "" : "disabled"}>
        ${x.title}
        <button type="button" class="dep-remove" data-action="dep-remove" data-id="${x.id}" aria-label="Remove ${x.title}">
          ${icon("close")}
        </button>
      </li>
    `)}</ul>
    <input type="search" id="dep-search" role="combobox" aria-controls="dep-options" aria-autocomplete="list" placeholder="Search tasks" autocomplete="off">
    <ul class="dep-options" id="dep-options" role="listbox" aria-label="Tasks">${options.map((x) => html`
      <li role="option" class="dep-option" data-action="dep-add" data-id="${x.id}" data-title="${x.title.toLowerCase()}" tabindex="-1" ${on(x.id) ? "hidden" : ""}>
        ${x.title}
      </li>
    `)}
    <li class="dep-none muted small" hidden>No matching task</li>
    </ul>
  </div>`;
}

/** Shows the options that are not chosen and contain the query (case-insensitive). */
export function filterDeps(picker: HTMLElement, query: string): void {
  const q = query.trim().toLowerCase();
  let shown = 0;
  for (const o of picker.querySelectorAll<HTMLElement>(".dep-option")) {
    const chosen = !picker.querySelector<HTMLElement>(`.dep-chip[data-id="${CSS.escape(o.dataset.id ?? "")}"]`)?.hidden;
    o.hidden = chosen || !(o.dataset.title ?? "").includes(q);
    if (!o.hidden) shown++;
  }
  const none = picker.querySelector<HTMLElement>(".dep-none");
  if (none) none.hidden = shown > 0;
}

/** Chooses (on) or drops (off) one task, then clears the search and puts the caret back in it. */
export function setDep(picker: HTMLElement, id: string, on: boolean): void {
  const chip = picker.querySelector<HTMLElement>(`.dep-chip[data-id="${CSS.escape(id)}"]`);
  if (!chip) return;
  chip.hidden = !on;
  const input = chip.querySelector<HTMLInputElement>("input[name='deps']");
  if (input) input.disabled = !on;
  const search = picker.querySelector<HTMLInputElement>("#dep-search");
  if (search) {
    search.value = "";
    search.focus();
  }
  filterDeps(picker, "");
}
