// ARTIFACTS: a file or long code block from the chat in a side pane (a sheet on phones), the way
// the Claude app shows artifacts: one header row (name, Preview/Code, Copy, Download, close), then
// the content. HTML and SVG preview in a sandboxed frame (sandbox.html: an opaque origin with no
// network), Markdown renders, everything else is highlighted code.
import { html, mount } from "../core/html";
import { highlightHtml, saveText } from "../core/codeblocks";
import { renderMarkdown } from "../core/markdown";
import { fileName, lineCount, previewDoc, viewOf, type Artifact } from "../core/artifact";
import { t } from "../core/i18n";
import { icon } from "./icons";

let current: { pane: HTMLElement; a: Artifact; tab: "preview" | "code"; opener: HTMLElement | null; stop: () => void } | undefined;

/** The highlighter's name for a language as written. */
const hlLang = (l: string): string => ({ htm: "html", svg: "xml", md: "markdown", sh: "bash", yml: "yaml" } as Record<string, string>)[l.toLowerCase()] ?? l.toLowerCase();

function body(a: Artifact, tab: "preview" | "code"): HTMLElement {
  const view = viewOf(a.lang);
  const box = document.createElement("div");
  box.className = "art-body";
  if (tab === "preview" && view === "html") {
    const frame = document.createElement("iframe");
    frame.className = "art-frame";
    frame.title = t("artPreview");
    frame.setAttribute("sandbox", "allow-scripts");
    frame.setAttribute("referrerpolicy", "no-referrer");
    const doc = previewDoc(a);
    const onMsg = (e: MessageEvent) => {
      if (e.source === frame.contentWindow && (e.data as { kkSandboxReady?: boolean } | null)?.kkSandboxReady) {
        frame.contentWindow?.postMessage({ kkArtifact: doc }, "*"); // the frame's origin is opaque: "*" is the only target
      }
    };
    window.addEventListener("message", onMsg);
    (frame as HTMLIFrameElement & { kkStop?: () => void }).kkStop = () => window.removeEventListener("message", onMsg);
    frame.src = "sandbox.html";
    box.append(frame);
  } else if (tab === "preview" && view === "markdown") {
    const md = document.createElement("div");
    md.className = "art-md body";
    md.append(renderMarkdown(a.text));
    box.append(md);
  } else {
    const pre = document.createElement("pre");
    pre.className = "art-code";
    const code = document.createElement("code");
    code.className = "hljs";
    mount(code, highlightHtml(a.text, hlLang(a.lang)));
    pre.append(code);
    box.append(pre);
  }
  return box;
}

function render(): void {
  if (!current) return;
  const { pane, a, tab } = current;
  pane.querySelectorAll<HTMLIFrameElement & { kkStop?: () => void }>("iframe").forEach((f) => f.kkStop?.());
  const previewable = viewOf(a.lang) !== "code";
  const lines = lineCount(a.text);
  mount(pane, html`
    <header class="art-head">
      <h2 class="art-title" id="art-title" title="${a.title}">${a.title.split(/[\\/]/).pop() || a.title}</h2>
      <span class="art-meta">${a.lang} · ${t("artLines", { n: lines })}</span>
      ${previewable ? html`<span class="art-tabs" role="tablist" aria-label="${t("artView")}">
        <button type="button" role="tab" data-art="preview" aria-selected="${tab === "preview" ? "true" : "false"}">${t("artPreview")}</button>
        <button type="button" role="tab" data-art="code" aria-selected="${tab === "code" ? "true" : "false"}">${t("artCode")}</button>
      </span>` : ""}
      <span class="art-tools">
        <button type="button" class="icon-btn" data-art="copy" aria-label="${t("artCopy")}" title="${t("artCopy")}">${icon("copy")}</button>
        <button type="button" class="icon-btn" data-art="download" aria-label="${t("artDownload")}" title="${t("artDownload")}">${icon("download")}</button>
        <button type="button" class="icon-btn" data-art="close" aria-label="${t("artClose")}" title="${t("artClose")}">${icon("close")}</button>
      </span>
    </header>`);
  pane.append(body(a, tab));
}

/** Opens (or replaces) the pane with one artifact. HTML, SVG and Markdown open on the preview. */
export function openArtifact(a: Artifact, opener?: HTMLElement | null): void {
  let pane = current?.pane;
  if (!pane) {
    pane = document.createElement("aside");
    pane.id = "artifact-pane";
    pane.className = "artifact-pane";
    pane.setAttribute("role", "dialog");
    pane.setAttribute("aria-modal", "false");
    pane.setAttribute("aria-labelledby", "art-title");
    pane.tabIndex = -1;
    document.body.append(pane);
    document.body.classList.add("has-artifact");
    pane.addEventListener("click", (ev) => {
      const b = (ev.target as HTMLElement).closest<HTMLElement>("[data-art]");
      if (!b || !current) return;
      const what = b.dataset.art;
      if (what === "close") closeArtifact();
      else if (what === "preview" || what === "code") { current.tab = what; render(); pane?.querySelector<HTMLElement>(`[data-art="${what}"]`)?.focus(); }
      else if (what === "copy") {
        void navigator.clipboard.writeText(current.a.text).then(() => { b.classList.add("done"); setTimeout(() => b.classList.remove("done"), 1500); }, () => undefined);
      } else if (what === "download") saveText(current.a.text, fileName(current.a));
    });
  }
  current?.stop();
  if (opener) rememberOpener(opener);
  const esc = (ev: KeyboardEvent) => { if (ev.key === "Escape" && !ev.defaultPrevented) closeArtifact(); };
  document.addEventListener("keydown", esc);
  current = { pane, a, tab: viewOf(a.lang) === "code" ? "code" : "preview", opener: opener ?? current?.opener ?? null, stop: () => document.removeEventListener("keydown", esc) };
  render();
  pane.focus();
}

export function closeArtifact(): void {
  if (!current) return;
  const { pane, opener, stop } = current;
  current = undefined;
  stop();
  pane.querySelectorAll<HTMLIFrameElement & { kkStop?: () => void }>("iframe").forEach((f) => f.kkStop?.());
  pane.remove();
  document.body.classList.remove("has-artifact");
  focusBack(opener);
}

/** Focus goes back to the button that opened the pane, or to its twin when a live redraw replaced it
 *  (the same action and id, at the same place among its kind). */
function focusBack(opener: HTMLElement | null): void {
  if (!opener) return;
  if (opener.isConnected) { opener.focus(); return; }
  const key = openerKey;
  if (!key) return;
  const sel = `[data-action="${CSS.escape(key.action)}"]${key.id ? `[data-id="${CSS.escape(key.id)}"]` : ""}`;
  document.querySelectorAll<HTMLElement>(sel)[key.index]?.focus();
}
let openerKey: { action: string; id: string; index: number } | undefined;
function rememberOpener(el: HTMLElement | null | undefined): void {
  if (!el?.dataset.action) { openerKey = undefined; return; }
  const action = el.dataset.action;
  const id = el.dataset.id ?? "";
  const sel = `[data-action="${CSS.escape(action)}"]${id ? `[data-id="${CSS.escape(id)}"]` : ""}`;
  openerKey = { action, id, index: [...document.querySelectorAll(sel)].indexOf(el) };
}

export const artifactOpen = (): boolean => current !== undefined;
