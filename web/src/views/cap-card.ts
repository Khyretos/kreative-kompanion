import { html, type SafeHtml } from "../core/html";
import { icon } from "./icons";

export function capHead(iconName: string, title: SafeHtml | string, chip: string, sub?: SafeHtml | string): SafeHtml {
  return html`<span class="cap-head"><span class="cap-icon">${icon(iconName)}</span><span class="task-title cap-title">${title}</span><span class="chip state">${chip}</span></span>${sub ? html`<span class="cap-sub">${sub}</span>` : ""}`;
}

export function capStep(text: string, limit = 120): SafeHtml {
  if (text.length <= limit) {
    return html`<span class="task-step">${text}</span>`;
  }
  return html`<span class="task-step cap-clamp">${text}</span><details class="cap-more"><summary>Show all</summary><p>${text}</p></details>`;
}
