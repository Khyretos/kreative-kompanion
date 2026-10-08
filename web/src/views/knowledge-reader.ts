// KNOW-01: read a knowledge collection's documents in a dialog: a name filter, the list in pages of 50
// and the chosen document's text next to it (Markdown rendered, anything else as plain text).
import { renderMarkdown } from "../core/markdown";
import type { KDoc } from "./knowledge";

export interface KnowledgeReaderApi {
  knowledgeDocs(id: string, q: string, offset: number): Promise<{ documents: KDoc[]; total: number }>;
  knowledgeDoc(id: string, doc: number): Promise<{ id: number; name: string; text: string }>;
}

const el = <K extends keyof HTMLElementTagNameMap>(tag: K, props: Record<string, unknown> = {}, ...kids: (Node | string)[]) => {
  const e = Object.assign(document.createElement(tag), props);
  e.append(...kids);
  return e;
};

/** Opens the reader for one collection; `showError` gets what went wrong. */
export function openKnowledgeReader(api: KnowledgeReaderApi, id: string, title: string, showError: (e: unknown) => void): void {
  const close = el("button", { type: "button", className: "icon-btn viewer-close", textContent: "×" });
  close.setAttribute("aria-label", "Close");
  const search = el("input", { type: "search", className: "kn-reader-search", placeholder: "Find a document by name" });
  search.setAttribute("aria-label", "Find a document by name");
  const list = el("ul", { className: "kn-reader-list" });
  const more = el("button", { type: "button", className: "btn small", textContent: "Show more" });
  const count = el("small", { className: "muted" });
  const side = el("div", { className: "kn-reader-side" }, search, count, list, more);
  const docName = el("h3", { className: "kn-reader-name", textContent: "Pick a document" });
  const text = el("div", { className: "kn-reader-text md" }, el("p", { className: "muted", textContent: "Pick a document on the left to read it." }));
  const main = el("div", { className: "kn-reader-main" }, docName, text);
  const head = el("header", { className: "viewer-head" }, el("h2", { textContent: title }), close);
  const dialog = el("dialog", { className: "asset-viewer kn-reader" }, head, el("div", { className: "viewer-body kn-reader-body" }, side, main));
  dialog.setAttribute("aria-label", `Documents in ${title}`);
  document.body.append(dialog);

  let offset = 0;
  let query = "";
  let token = 0;
  const load = (reset: boolean) => {
    const mine = ++token;
    if (reset) { offset = 0; list.replaceChildren(); }
    api.knowledgeDocs(id, query, offset).then((r) => {
      if (mine !== token) return;
      for (const d of r.documents) {
        const b = el("button", { type: "button", className: "kn-reader-doc", textContent: d.name, title: d.name });
        b.addEventListener("click", () => open(d, b));
        list.append(el("li", {}, b));
      }
      offset += r.documents.length;
      count.textContent = `${offset} of ${r.total}`;
      more.hidden = offset >= r.total;
      if (r.total === 0) list.append(el("li", { className: "muted small", textContent: "No document matches." }));
    }, showError);
  };
  const open = (d: KDoc, b: HTMLElement) => {
    list.querySelectorAll(".kn-reader-doc[aria-current]").forEach((x) => x.removeAttribute("aria-current"));
    b.setAttribute("aria-current", "true");
    docName.textContent = d.name;
    text.replaceChildren(el("p", { className: "muted", textContent: "Loading…" }));
    api.knowledgeDoc(id, d.id).then((r) => {
      if (docName.textContent !== r.name) return;
      if (/\.(md|markdown)$/i.test(r.name)) text.replaceChildren(renderMarkdown(r.text));
      else text.replaceChildren(el("pre", { className: "kn-reader-plain", textContent: r.text }));
      text.scrollTop = 0;
    }, (e) => { text.replaceChildren(); showError(e); });
  };
  let timer: ReturnType<typeof setTimeout> | undefined;
  search.addEventListener("input", () => {
    clearTimeout(timer);
    timer = setTimeout(() => { query = search.value.trim(); load(true); }, 250);
  });
  more.addEventListener("click", () => load(false));
  close.addEventListener("click", () => dialog.close());
  dialog.addEventListener("close", () => dialog.remove());
  dialog.addEventListener("click", (ev) => { if (ev.target === dialog) dialog.close(); });
  dialog.showModal();
  load(true);
}
