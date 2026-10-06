// lesson proposals in the project thread; nothing a model drafts becomes a rule before the user accepts it.
import { html, type SafeHtml } from "../core/html";
import type { Lesson } from "../api/client";

export function renderLessons(lessons: Lesson[]): SafeHtml {
  const proposed = lessons.filter((l) => l.state === "proposed");

  if (proposed.length === 0) {
    return html``;
  }

  return html`
    <section class="pc-actions lessons" aria-label="Proposed lessons">
      ${proposed.map((l) => {
        return html`
          <article class="pc-action lesson" data-id="${l.id}">
            <header class="card-kind">
              <span>Lesson</span>
              <span class="need">for ${l.card} · by ${l.proposedBy || "the reviewer"}</span>
            </header>
            <div class="lesson-layer">
              <label class="small" for="lesson-layer-${l.id}">Layer</label>
              <select id="lesson-layer-${l.id}" name="layer">
                ${l.layer === "general" ? html`<option value="general" selected>General (any project, published to kompas-skills)</option><option value="private">Private (this setup only)</option>` : html`<option value="general">General (any project, published to kompas-skills)</option><option value="private" selected>Private (this setup only)</option>`}
              </select>
            </div>
            <span class="muted small">From the review: ${l.finding}</span>
            <label class="small" for="lesson-${l.id}">Lesson (you can edit it)</label>
            <textarea id="lesson-${l.id}" name="lesson" rows="2" data-id="${l.id}">${l.text}</textarea>
            <div class="actions">
              <button class="btn small primary" data-action="lesson-accept" data-id="${l.id}">Accept</button>
              <button class="btn small" data-action="lesson-dismiss" data-id="${l.id}">Dismiss</button>
            </div>
          </article>
        `;
      })}
    </section>
  `;
}
