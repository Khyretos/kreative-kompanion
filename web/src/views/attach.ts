// CHAT-08: files attached to a chat message. The composer keeps the chosen files in memory
// until Send (a new chat has no id to upload to yet); a sent message shows its files as chips.
import { html, SafeHtml } from "../core/html";
import { Attached, AttachedKind, kindOf, sizeLabel } from "../core/attachments";
import { icon } from "./icons";

export const MAX_FILES = 8;

export interface Pending { file: File; kind: AttachedKind; thumb?: string }

/** The files that may be added (a message takes up to 8) and what was refused, with the reason.
 * `maxMb` is the user's largest attachment from Settings; 0 means no limit. */
export function accept(have: Pending[], files: File[], maxMb = 0): { added: Pending[]; refused: string[] } {
  const max = maxMb * 1024 * 1024;
  const added: Pending[] = [];
  const refused: string[] = [];
  for (const file of files) {
    const kind = kindOf(file.name);
    if (file.size === 0) refused.push(`${file.name}: the file is empty`);
    else if (max > 0 && file.size > max) refused.push(`${file.name}: larger than the ${sizeLabel(max)} set in Settings (Largest attachment)`);
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

/** What is under a sent message: pictures as previews that open large on a click (IMG-VIEW),
 * other files as chips that save the file under the name it had (a document is saved as the text the model read). */
export function renderSent(chatId: string, files: Attached[]): SafeHtml {
  if (!files.length) return html``;
  const url = (f: Attached) => `api/chats/${encodeURIComponent(chatId)}/files/${encodeURIComponent(f.file)}`;
  const saveAs = (f: Attached) => f.kind === "file" ? f.name : f.name.replace(/\.[^.]+$/, "") + ".txt";
  return html`<div class="attach-sent">${files.map((f) => f.kind === "image" ? html`
    <button type="button" class="attach-img" data-action="img-open" data-src="${url(f)}" data-name="${f.name}" title="${f.name}" aria-label="Open ${f.name}">
      <img src="${url(f)}" alt="${f.name}" loading="lazy"><span class="attach-zoom" aria-hidden="true">${icon("search")}</span></button>` : html`
    <a class="chip attach-chip" href="${url(f)}?name=${encodeURIComponent(saveAs(f))}" target="_blank" rel="noopener" download="${saveAs(f)}"
      title="${f.name}">${icon("file")}<span class="attach-name">${f.name}</span></a>`)}</div>`;
}
