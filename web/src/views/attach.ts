// CHAT-08: files attached to a chat message. The composer keeps the chosen files in memory
// until Send (a new chat has no id to upload to yet); a sent message shows its files as chips.
import { html, SafeHtml } from "../core/html";
import { Attached, kindOf, sizeLabel } from "../core/attachments";
import { icon } from "./icons";

export const MAX_FILES = 8;
const MAX_IMAGE = 8 * 1024 * 1024;
const MAX_TEXT = 25 * 1024 * 1024;

export interface Pending { file: File; kind: "image" | "text"; thumb?: string }

/** The files that may be added (a message takes up to 8) and what was refused, with the reason. */
export function accept(have: Pending[], files: File[]): { added: Pending[]; refused: string[] } {
  const added: Pending[] = [];
  const refused: string[] = [];
  for (const file of files) {
    const kind = kindOf(file.name);
    if (!kind) refused.push(`${file.name}: pictures (PNG, JPG, WebP), PDFs, text and code files only`);
    else if (file.size === 0) refused.push(`${file.name}: the file is empty`);
    else if (file.size > (kind === "image" ? MAX_IMAGE : MAX_TEXT)) refused.push(`${file.name}: larger than ${sizeLabel(kind === "image" ? MAX_IMAGE : MAX_TEXT)}`);
    else if (have.length + added.length >= MAX_FILES) refused.push(`${file.name}: a message takes up to ${MAX_FILES} files`);
    else if (have.some((p) => p.file.name === file.name && p.file.size === file.size)) continue;
    else added.push({ file, kind, thumb: kind === "image" ? URL.createObjectURL(file) : undefined });
  }
  return { added, refused };
}

export function release(list: Pending[]): void {
  for (const p of list) if (p.thumb) URL.revokeObjectURL(p.thumb);
}

function glyph(kind: string, thumb?: string): SafeHtml {
  return thumb ? html`<img class="attach-thumb" src="${thumb}" alt="">` : icon(kind === "image" ? "image" : "file");
}

/** The chips above the composer's text field, each with a remove button. */
export function renderPending(list: Pending[]): SafeHtml {
  return html`${list.map((p, i) => html`
    <span class="chip attach-chip" title="${p.file.name}">${glyph(p.kind, p.thumb)}<span class="attach-name">${p.file.name}</span>
      <span class="attach-size">${sizeLabel(p.file.size)}</span>
      <button type="button" class="attach-x" data-action="attach-remove" data-idx="${i}" aria-label="Remove ${p.file.name}">${icon("close")}</button></span>`)}`;
}

/** The chips under a sent message: a click opens or saves the file. */
export function renderSent(chatId: string, files: Attached[]): SafeHtml {
  if (!files.length) return html``;
  return html`<div class="attach-sent">${files.map((f) => html`
    <a class="chip attach-chip" href="api/chats/${encodeURIComponent(chatId)}/files/${encodeURIComponent(f.file)}" target="_blank" rel="noopener" download="${f.kind === "text" ? f.name.replace(/\.[^.]+$/, "") + ".txt" : f.name}"
      title="${f.name}">${icon(f.kind === "image" ? "image" : "file")}<span class="attach-name">${f.name}</span></a>`)}</div>`;
}
