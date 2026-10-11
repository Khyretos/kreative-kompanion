import { html, type SafeHtml } from "../core/html";
import { icon } from "./icons";

/** NAV-01: the head of every full page (Capabilities, Studio, Settings; Assets builds its own
 *  with the same ☰). On a phone the ☰ opens the sidebar, like in a chat; back goes to the chat. */
export function pageHead(title: string, sub: string, id = ""): SafeHtml {
  return html`
    <header class="caps-head page-head">
      <button class="icon-btn only-phone" data-action="pane" data-pane="left" aria-label="Projects and chats">${icon("menu")}</button>
      <div class="caps-title">
        <h1 ${id ? html`id="${id}"` : ""}>${title}</h1>
        <p class="muted">${sub}</p>
      </div>
    </header>`;
}
