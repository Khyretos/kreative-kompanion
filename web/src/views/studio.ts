// STU-01/02: the Studio section: pick a type, describe it, make it; your images, videos and sounds.
import { html, type SafeHtml } from "../core/html";
import { SIZE_LABELS, typeCard, typeDetail } from "./studio-cards";

export interface StudioType { name: string; label: string; hint: string; sizes: string[]; order: number; warning?: string; audio?: "music" | "sfx"; seconds?: { min: number; max: number; default: number }; ratings?: string[]; adultRatings?: string[]; face?: boolean }
export interface StudioRun { id: string; type: string; gpu: string; prompt: string; size: string; seconds?: number | null; state: "running" | "done" | "failed"; error: string | null; files: string[]; startedAt: string; endedAt: string | null }
export interface StudioForm { type: string; size: string; count: 1 | 4; busy: boolean; error?: string }

export function renderStudioMake(types: StudioType[] | undefined, form: StudioForm, prompt: string, lyrics: string, seconds: string, rating: string, adult: boolean, faceName: string, faceWeight: string): SafeHtml {
  if (!types) return html`<p class="muted">Loading the image types…</p>`;
  const chosen = types.find((t) => t.name === form.type) ?? types[0];
  const audio = chosen?.audio;
  const single = !!audio || chosen?.name === "video";
  const sizes = chosen?.sizes ?? [];
  const ratings = (chosen?.ratings ?? []).filter((r) => adult || !(chosen?.adultRatings ?? []).includes(r));

  return html`<section class="studio-make" aria-labelledby="studio-make-h">
    <h2 id="studio-make-h" class="label">Make</h2>
    <div class="studio-pick"><div class="studio-types" role="radiogroup" aria-label="What to make">${types.map((t) => typeCard(t, t.name === chosen?.name))}</div>${typeDetail(chosen)}</div>
    <form class="studio-form">
      <label for="studio-prompt">${audio ? "What should it sound like?" : "What should it show?"}</label>
      <textarea id="studio-prompt" name="prompt" rows="3" maxlength="${audio ? 1000 : 500}" placeholder="${audio === "music" ? "calm lofi piano loop for a cozy game menu" : audio === "sfx" ? "a wooden door creaking open slowly" : "a cheerful fox adventurer with a green scarf"}">${prompt}</textarea>
      ${audio === "music" ? html`<label for="studio-lyrics">Lyrics (optional)</label><textarea id="studio-lyrics" name="lyrics" rows="3" maxlength="3000" placeholder="Leave empty for an instrumental">${lyrics}</textarea>` : ""}
      ${chosen?.face ? html`<div class="studio-face"><label for="studio-face">Face photo (optional)</label><input id="studio-face" name="face" type="file" accept="image/jpeg,image/png,image/webp">${faceName ? html`<span class="small">${faceName} <button type="button" class="btn small" data-action="studio-face-clear">Remove</button></span>` : ""}<label for="studio-face-weight">Likeness <input id="studio-face-weight" name="face_weight" type="range" min="0" max="1.2" step="0.05" value="${faceWeight || "0.85"}"></label><p class="muted small">The character takes on this face. The photo is deleted after the run.</p></div>` : ""}
      <div class="studio-options">
        ${ratings.length ? html`<label class="studio-rating" for="studio-rating">Rating <select id="studio-rating" name="rating">${ratings.map((r) => html`<option value="${r}" ${r === rating ? "selected" : ""}>${r[0].toUpperCase() + r.slice(1)}</option>`)}</select></label>` : ""}
        ${audio && chosen?.seconds ? html`<label class="studio-length" for="studio-seconds">Length <input id="studio-seconds" name="seconds" type="number" min="${chosen.seconds.min}" max="${chosen.seconds.max}" value="${seconds || String(chosen.seconds.default)}"> s</label>` : ""}
        ${sizes.length ? html`<span class="seg" role="group" aria-label="Size">${sizes.map((z) => html`<button type="button" data-action="studio-size" data-size="${z}" aria-pressed="${String(z === form.size)}">${SIZE_LABELS[z] ?? z}</button>`)}</span>` : ""}
        ${single ? "" : html`<label class="studio-count"><input type="checkbox" name="four" ${form.count === 4 ? "checked" : ""}> Make 4</label>`}
        <button class="btn primary" type="submit" ${form.busy ? "disabled" : ""}>${form.busy ? "Starting…" : "Make"}</button>
      </div>
      ${form.error ? html`<p class="error small" role="alert">${form.error}</p>` : ""}
    </form>
  </section>`;
}

export function renderStudioLibrary(types: StudioType[] | undefined, runs: StudioRun[] | undefined): SafeHtml {
  const label = (name: string) => types?.find((t) => t.name === name)?.label ?? name;
  const list = runs === undefined ? html`<p class="muted small">Loading your results…</p>` : runs.length === 0 ? html`<p class="muted small">Nothing yet. What you make appears here as soon as it is ready.</p>` : html`<ul class="studio-runs">${runs.map((r) => runCard(r, label(r.type)))}</ul>`;
  return html`<section class="studio-library" aria-labelledby="studio-lib-h"><h2 id="studio-lib-h" class="label">Your results</h2>${list}</section>`;
}

function runCard(r: StudioRun, typeLabel: string): SafeHtml {
  const isAudio = r.type === "music" || r.type === "sfx";
  const what = isAudio ? `${r.seconds ?? "?"} s` : SIZE_LABELS[r.size] ?? r.size;
  let pictures: SafeHtml;
  if (r.state === "running") {
    pictures = html`<div class="studio-thumb pending" role="status">Making…</div>`;
  } else if (r.state === "failed") {
    pictures = html`<div class="studio-thumb failed">Failed</div><p class="error small">${r.error ?? "Something went wrong."}</p>`;
  } else if (isAudio) {
    pictures = html`<div class="studio-audio"><audio controls preload="none" src="${r.files[0] ?? ""}"></audio>${r.files[1] ? html`<a class="small" href="${r.files[1]}" download>WAV</a>` : ""}</div>`;
  } else if (r.type === "video") {
    pictures = html`<div class="studio-pics">${r.files.map((src) => html`<video class="studio-video" controls preload="metadata" src="${src}" aria-label="${typeLabel}: ${r.prompt}"></video>`)}</div>`;
  } else {
    pictures = html`<div class="studio-pics">${r.files.map((src) => html`<button type="button" class="studio-thumb" data-action="studio-open" data-src="${src}"><img src="${src}" alt="${typeLabel}: ${r.prompt}" loading="lazy"></button>`)}</div>`;
  }
  return html`<li class="studio-run s-${r.state}">${pictures}<p class="small"><strong>${typeLabel} · ${what}</strong> · ${r.prompt}</p></li>`;
}
