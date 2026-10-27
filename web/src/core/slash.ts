// SLASH (Kees 10-27): "/" commands in the message box. They act at once (pick a computer, set
// the effort, switch automatic mode...) without sending anything, so the work keeps flowing;
// a few are prompt starters that fill the box. The list filters as you type; arrows, Tab and
// Enter pick, Esc closes.
import { html, type SafeHtml } from "./html";

export interface SlashCommand {
  name: string; // what is typed, without the slash: "effort high"
  hint: string;
  /** Runs it; or, for a prompt starter, the text the message box gets. */
  run?: () => void;
  fill?: string;
}

/** The commands whose name starts with what was typed after the slash (words in any case). */
export function matchSlash(all: SlashCommand[], typed: string): SlashCommand[] {
  const q = typed.replace(/^\//, "").trimStart().toLowerCase().replace(/\s+/g, " ");
  const hits = all.filter((c) => c.name.startsWith(q));
  // "/eff hi" still finds "effort high": every typed word starts a word of the name, in order.
  if (hits.length || !q.includes(" ")) return hits.length ? hits : all.filter((c) => c.name.split(" ").some((w) => w.startsWith(q)));
  const words = q.split(" ");
  return all.filter((c) => { const n = c.name.split(" "); return words.every((w, i) => n[i]?.startsWith(w)); });
}

/** Whether the message box text is a command being typed: one line starting with "/". */
export function typingSlash(text: string): boolean {
  return /^\/[\w -]*$/.test(text);
}

export function renderSlash(list: SlashCommand[], active: number): SafeHtml {
  if (!list.length) return html`<p class="slash-empty muted small">No command like that. Type / to see them all.</p>`;
  return html`${list.map((c, i) => html`<button type="button" role="option" class="slash-item" data-idx="${i}" aria-selected="${i === active ? "true" : "false"}">
    <code>/${c.name}</code><span class="slash-hint">${c.hint}</span></button>`)}`;
}
