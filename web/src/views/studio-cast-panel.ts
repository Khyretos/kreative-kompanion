import { html, type SafeHtml } from "../core/html";
import { CAST_MIN, CAST_MAX } from "./studio-cast";

export function castPanel(p: string[]): SafeHtml {
  const last = p.length - 1;
  const people = p.map((text, i): SafeHtml => {
    const canMoveLeft = i > 0;
    const canMoveRight = i < last;
    const canRemove = p.length > CAST_MIN;
    return html`<div class="studio-cast-person">
      <label for="studio-person-${i}">Person ${i + 1} (left to right)</label>
      <textarea id="studio-person-${i}" name="person-${i}" rows="2" maxlength="300" placeholder="1girl, long red hair, green eyes, school uniform">${text}</textarea>
      ${canMoveLeft ? html`<button type="button" data-action="studio-cast-move" data-index="${i}" data-dir="-1">Move left</button>` : ""}
      ${canMoveRight ? html`<button type="button" data-action="studio-cast-move" data-index="${i}" data-dir="1">Move right</button>` : ""}
      ${canRemove ? html`<button type="button" data-action="studio-cast-remove" data-index="${i}">Remove</button>` : ""}
    </div>`;
  });

  const addBtn = p.length < CAST_MAX
    ? html`<button type="button" data-action="studio-cast-add">Add a person</button>`
    : "";

  return html`<fieldset class="studio-cast">
    <legend>People</legend>
    ${people}
    ${addBtn}
  </fieldset>`;
}
