import { html, type SafeHtml } from "../core/html";
import { icon } from "./icons";
import type { StudioType } from "./studio";

export const SIZE_LABELS: Record<string, string> = { square: "Square", wide: "Wide", tall: "Tall" };

export function typeCard(t: StudioType, on: boolean): SafeHtml {
  const kind = t.audio ? "Audio" : t.name === "video" ? "Video" : "Image";
  const badges: SafeHtml[] = [
    html`<span class="studio-badge">${kind}</span>`,
    t.face ? html`<span class="studio-badge">Face</span>` : null,
    t.ratings?.length ? html`<span class="studio-badge">Rating</span>` : null,
    t.warning ? html`<span class="studio-badge warn" title="Licence warning">${icon("alert", "Licence warning")}</span>` : null,
  ].filter((x) => x !== null);
  return html`<button type="button" class="studio-type" role="radio" aria-checked="${String(on)}" tabindex="${on ? "0" : "-1"}" data-action="studio-type" data-type="${t.name}"><strong>${t.label}</strong><span class="studio-badges">${badges}</span></button>`;
}

export function typeDetail(t: StudioType | undefined): SafeHtml {
  if (!t) return html``;
  const takes = [
    t.face ? "a face photo (optional)" : "",
    t.ratings?.length ? "a rating" : "",
    t.audio && t.seconds ? `a length of ${t.seconds.min} to ${t.seconds.max} s` : "",
  ].filter((x) => x !== "");
  return html`<aside class="studio-type-detail" id="studio-type-detail" aria-live="polite" data-type="${t.name}">
    <h3>${t.label}</h3>
    <p>${t.hint}</p>
    ${t.sizes.length ? html`<p class="small"><span class="muted">Sizes:</span> ${t.sizes.map((z): string => SIZE_LABELS[z] ?? z).join(", ")}</p>` : ""}
    ${takes.length ? html`<p class="small"><span class="muted">Takes:</span> ${takes.join(", ")}</p>` : ""}
    ${t.warning ? html`<p class="small studio-warning">Licence warning: ${t.warning}</p>` : ""}
  </aside>`;
}
