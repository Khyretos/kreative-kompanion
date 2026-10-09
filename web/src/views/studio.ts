// STU-01/02: the Studio section: pick a type, describe it, make it; your images, videos and sounds.
import { html, type SafeHtml } from "../core/html";
import { SIZE_LABELS, typeCard, typeDetail } from "./studio-cards";
import { icon } from "./icons";

export interface StudioType { name: string; label: string; hint: string; sizes: string[]; order: number; warning?: string; audio?: "music" | "sfx"; seconds?: { min: number; max: number; default: number }; ratings?: string[]; adultRatings?: string[]; face?: boolean; negative?: string; helmet?: boolean }
export interface StudioRun { id: string; type: string; gpu: string; prompt: string; size: string; seconds?: number | null; state: "running" | "done" | "failed"; error: string | null; waiting?: string | null; helmet?: "on" | "off" | null; pair?: string | null; files: string[]; startedAt: string; endedAt: string | null }
export interface StudioForm { type: string; size: string; count: 1 | 4; busy: boolean; error?: string }

export function renderStudioMake(types: StudioType[] | undefined, form: StudioForm, prompt: string, lyrics: string, seconds: string, rating: string, adult: boolean, faceName: string, faceWeight: string, negative = "", helmet = ""): SafeHtml {
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
      ${chosen?.negative !== undefined ? html`<details class="studio-neg" ${negative ? "open" : ""}><summary>Leave out</summary><textarea id="studio-negative" name="negative" rows="2" maxlength="500" aria-label="Leave out (negative prompt)" placeholder="blurry, extra fingers, hats">${negative}</textarea><p class="muted small">Things the picture should not show.${chosen.negative ? ` Already left out: ${chosen.negative}.` : ""}</p></details>` : ""}
      ${chosen?.helmet ? html`<details class="studio-helmet" ${helmet ? "open" : ""}><summary>Has a helmet</summary><label for="studio-helmet">What does the helmet look like?</label><input id="studio-helmet" name="helmet" type="text" maxlength="300" value="${helmet}" placeholder="hooded helmet with glowing yellow eyes"><p class="muted small">Makes two versions with the same seed: helmet on, and without it so the face shows.</p></details>` : ""}
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

export function renderStudioLibrary(
  types: StudioType[] | undefined,
  runs: StudioRun[] | undefined,
  projects: { id: string; name: string }[],
  sent: Record<string, string[]>,
  picked: string[] = [],
  hidden: string[] = [],
  sides: Record<string, "on" | "off"> = {}
): SafeHtml {
  const label = (name: string) => types?.find((t) => t.name === name)?.label ?? name;
  const shown = runs?.filter((r) => !hidden.includes(r.id));
  const failed = shown?.filter((r) => r.state === "failed").length ?? 0;
  // STU-D1: a selection bar while runs are picked, else "Clear failed" when there are failed runs.
  const tools = picked.length
    ? html`<div class="studio-lib-tools" role="group" aria-label="Selected results"><span class="small" aria-live="polite">${String(picked.length)} selected</span><button type="button" class="btn small danger" data-action="studio-delete-picked">${icon("trash")} Delete ${String(picked.length)}</button><button type="button" class="btn small" data-action="studio-pick-clear">Cancel</button></div>`
    : failed
      ? html`<div class="studio-lib-tools"><button type="button" class="btn small" data-action="studio-clear-failed">Clear failed (${String(failed)})</button></div>`
      : "";
  const list =
    shown === undefined
      ? html`<p class="muted small">Loading your results…</p>`
      : shown.length === 0
        ? html`<p class="muted small">Nothing yet. What you make appears here as soon as it is ready.</p>`
        : html`<ul class="studio-runs">
            ${groupPairs(shown).map((g) => g.length === 2 ? pairCard(g[0], g[1], sides[g[0].pair ?? ""] ?? "on", (r) => runCard(r, label(r.type), projects, sent[r.id] ?? [], picked.includes(r.id))) : runCard(g[0], label(g[0].type), projects, sent[g[0].id] ?? [], picked.includes(g[0].id)))}
          </ul>`;
  return html`<section class="studio-library" aria-labelledby="studio-lib-h">
    <div class="studio-lib-head"><h2 id="studio-lib-h" class="label">Your results</h2>${tools}</div>
    ${list}
  </section>`;
}

export function runCard(
  r: StudioRun,
  typeLabel: string,
  projects: { id: string; name: string }[],
  sentTo: string[],
  picked = false
): SafeHtml {
  const isAudio = r.type === "music" || r.type === "sfx";
  const what = isAudio ? `${r.seconds ?? "?"} s` : SIZE_LABELS[r.size] ?? r.size;
  let pictures: SafeHtml;
  if (r.state === "running") {
    pictures = r.waiting
      ? html`<div class="studio-thumb pending" role="status" title="${r.waiting}">Waiting for room…</div><p class="small muted studio-wait">${r.waiting}</p>`
      : html`<div class="studio-thumb pending" role="status">Making…</div>`;
  } else if (r.state === "failed") {
    pictures = html`<div class="studio-thumb failed">Failed</div><p class="error small">${r.error ?? "Something went wrong."}</p>`;
  } else if (isAudio) {
    pictures = html`<div class="studio-audio"><audio controls preload="none" src="${r.files[0] ?? ""}"></audio>${r.files[1] ? html`<a class="small" href="${r.files[1]}" download>WAV</a>` : ""}</div>`;
  } else if (r.type === "video") {
    pictures = html`<div class="studio-pics">${r.files.map((src) => html`<video class="studio-video" controls preload="metadata" src="${src}" aria-label="${typeLabel}: ${r.prompt}"></video>`)}</div>`;
  } else {
    pictures = html`<div class="studio-pics">${r.files.map((src) => html`<button type="button" class="studio-thumb" data-action="studio-open" data-src="${src}"><img src="${src}" alt="${typeLabel}: ${r.prompt}" loading="lazy"></button>`)}</div>`;
  }
  const sent = sentTo.length > 0 ? html`<span class="chip studio-sent">In Assets: ${sentTo.join(", ")}</span>` : "";
  const menu = r.state === "done" && projects.length > 0
    ? html`<details class="studio-send"><summary>Send to Assets</summary><div class="studio-send-list" role="group" aria-label="Send to which project">${projects.map((p) => html`<button type="button" class="btn small" data-action="studio-send" data-run="${r.id}" data-project="${p.id}">${p.name}</button>`)}</div></details>`
    : "";
  // STU-D1: finished runs (done or failed) can be picked and deleted; a running one cannot.
  const finished = r.state === "done" || r.state === "failed";
  const top = finished
    ? html`<div class="studio-run-top"><button type="button" class="round-btn" role="checkbox" aria-checked="${picked ? "true" : "false"}" data-action="studio-pick" data-run="${r.id}" aria-label="Select" title="Select">${icon("check")}</button><button type="button" class="round-btn" data-action="studio-delete" data-run="${r.id}" aria-label="Delete" title="Delete">${icon("trash")}</button></div>`
    : "";
  return html`<li class="studio-run s-${r.state}${picked ? " picked" : ""}" data-helmet="${r.helmet ?? ""}">${top}${pictures}<p class="small"><strong>${typeLabel} · ${what}</strong> · ${r.prompt}</p>${sent}${menu}</li>`;
}

/** STU-C3: runs in list order; the two sides of a helmet pair become one group (helmet on first). */
export function groupPairs(runs: StudioRun[]): StudioRun[][] {
  const out: StudioRun[][] = [];
  const at = new Map<string, number>();
  for (const r of runs) {
    const i = r.pair ? at.get(r.pair) : undefined;
    if (i !== undefined && out[i].length === 1) {
      out[i].push(r);
      out[i].sort((a, b) => (a.helmet === "on" ? 0 : 1) - (b.helmet === "on" ? 0 : 1));
    } else {
      if (r.pair) at.set(r.pair, out.length);
      out.push([r]);
    }
  }
  return out;
}

/** STU-C3: one card for a helmet pair with a Helmet on / Face toggle; both runs stay in the DOM, CSS shows one. */
export function pairCard(on: StudioRun, off: StudioRun, side: "on" | "off", card: (r: StudioRun) => SafeHtml): SafeHtml {
  return html`<li class="studio-pair" data-pair="${on.pair ?? ""}" data-side="${side}"><span class="seg studio-side" role="group" aria-label="Helmet"><button type="button" data-action="studio-side" data-pair="${on.pair ?? ""}" data-side="on" aria-pressed="${String(side === "on")}">Helmet on</button><button type="button" data-action="studio-side" data-pair="${on.pair ?? ""}" data-side="off" aria-pressed="${String(side === "off")}">Face</button></span><ul class="studio-pair-runs">${card(on)}${card(off)}</ul></li>`;
}
