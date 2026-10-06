// STU-01: the Studio section: pick an image type, describe it, pick a size, make 1 or 4; your images.
import { html, type SafeHtml } from "../core/html";

export interface StudioType { name: string; label: string; hint: string; sizes: string[]; order: number }
export interface StudioRun { id: string; type: string; gpu: string; prompt: string; size: string; state: "running" | "done" | "failed"; error: string | null; files: string[]; startedAt: string; endedAt: string | null }
export interface StudioForm { type: string; size: string; count: 1 | 4; busy: boolean; error?: string }

const SIZE_LABELS: Record<string, string> = { square: "Square", wide: "Wide", tall: "Tall" };

export function renderStudioMake(types: StudioType[] | undefined, form: StudioForm, prompt: string): SafeHtml {
  if (!types) return html`<p class="muted">Loading the image types…</p>`;
  const chosen = types.find((t) => t.name === form.type) ?? types[0];
  const sizes = chosen?.sizes ?? ["square"];
  return html`<section class="studio-make" aria-labelledby="studio-make-h">
    <h2 id="studio-make-h" class="label">Make</h2>
    <div class="studio-types" role="radiogroup" aria-label="Image type">${types.map((t) => html`<button type="button" class="studio-type" role="radio" aria-checked="${String(t.name === chosen?.name)}" data-action="studio-type" data-type="${t.name}"><strong>${t.label}</strong><span class="muted small">${t.hint}</span></button>`)}</div>
    <form class="studio-form">
      <label for="studio-prompt">What should it show?</label>
      <textarea id="studio-prompt" name="prompt" rows="3" maxlength="500" placeholder="a cheerful fox adventurer with a green scarf">${prompt}</textarea>
      <div class="studio-options">
        <span class="seg" role="group" aria-label="Size">${sizes.map((z) => html`<button type="button" data-action="studio-size" data-size="${z}" aria-pressed="${String(z === form.size)}">${SIZE_LABELS[z] ?? z}</button>`)}</span>
        <label class="studio-count"><input type="checkbox" name="four" ${form.count === 4 ? "checked" : ""}> Make 4</label>
        <button class="btn primary" type="submit" ${form.busy ? "disabled" : ""}>${form.busy ? "Starting…" : "Make"}</button>
      </div>
      ${form.error ? html`<p class="error small" role="alert">${form.error}</p>` : ""}
    </form>
  </section>`;
}

export function renderStudioLibrary(types: StudioType[] | undefined, runs: StudioRun[] | undefined): SafeHtml {
  const label = (name: string) => types?.find((t) => t.name === name)?.label ?? name;
  const list = runs === undefined
    ? html`<p class="muted small">Loading your images…</p>`
    : runs.length === 0
      ? html`<p class="muted small">Nothing yet. Your images appear here as soon as they are ready.</p>`
      : html`<ul class="studio-runs">${runs.map((r) => runCard(r, label(r.type)))}</ul>`;
  return html`<section class="studio-library" aria-labelledby="studio-lib-h"><h2 id="studio-lib-h" class="label">Your images</h2>${list}</section>`;
}

function runCard(r: StudioRun, typeLabel: string): SafeHtml {
  let pictures: SafeHtml;
  if (r.state === "running") {
    pictures = html`<div class="studio-thumb pending" role="status">Making…</div>`;
  } else if (r.state === "failed") {
    pictures = html`<div class="studio-thumb failed">Failed</div><p class="error small">${r.error ?? "Something went wrong."}</p>`;
  } else {
    pictures = html`<div class="studio-pics">${r.files.map((src) => html`<button type="button" class="studio-thumb" data-action="studio-open" data-src="${src}"><img src="${src}" alt="${typeLabel}: ${r.prompt}" loading="lazy"></button>`)}</div>`;
  }
  return html`<li class="studio-run s-${r.state}">${pictures}<p class="small"><strong>${typeLabel} · ${SIZE_LABELS[r.size] ?? r.size}</strong> · ${r.prompt}</p></li>`;
}
