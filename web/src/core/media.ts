// CHAT-05: pictures and files in chat answers get the same Copy / Download buttons as code blocks.
// Only our own chat-file URLs get here (markdown.ts lets nothing else through).

const FILE = /^\/api\/chats\/[\w-]+\/files\/[0-9a-f-]{36}\.(blend|glb|zip|pdf|txt|json|csv|md)$/;
export const FILE_HREF = FILE;

function button(label: string, action: string, extra: Record<string, string> = {}): HTMLButtonElement {
  const b = document.createElement("button");
  b.type = "button";
  b.className = "btn small";
  b.dataset.action = action;
  for (const [k, v] of Object.entries(extra)) b.dataset[k] = v;
  b.textContent = label;
  return b;
}

/** A real file name (scene.blend) for the download, not the uuid in the URL. */
export function downloadName(text: string, ext: string): string {
  const base = text.replace(/[^\w. -]+/g, "_").replace(/^[. ]+/, "").slice(0, 80);
  if (!base) return `file.${ext}`;
  return base.toLowerCase().endsWith(`.${ext}`) ? base : `${base}.${ext}`;
}

/** Wraps every picture in a figure with Copy and Download, and turns file links into download cards. */
export function enhanceMedia(frag: DocumentFragment): void {
  for (const img of Array.from(frag.querySelectorAll("img"))) {
    const src = img.getAttribute("src") ?? "";
    const fig = document.createElement("figure");
    fig.className = "media";
    const bar = document.createElement("div");
    bar.className = "media-bar";
    const name = img.getAttribute("alt") || "image";
    const label = document.createElement("span");
    label.className = "media-name";
    label.textContent = name;
    bar.append(label, button("Copy", "media-copy", { src }), button("Download", "media-download", { src, name }));
    // A picture that does not load (the model made up the link) is not shown with buttons.
    img.addEventListener("error", () => fig.remove());
    img.replaceWith(fig);
    fig.append(img, bar);
  }
  for (const a of Array.from(frag.querySelectorAll("a"))) {
    const href = a.getAttribute("href") ?? "";
    if (!FILE.test(href)) continue;
    const ext = href.slice(href.lastIndexOf(".") + 1);
    const card = document.createElement("div");
    card.className = "media file";
    const label = document.createElement("span");
    label.className = "media-name";
    label.textContent = (a.textContent || `file.${ext}`).trim();
    const dl = document.createElement("a");
    dl.className = "btn small";
    dl.href = href;
    dl.setAttribute("download", downloadName(label.textContent ?? "", ext));
    dl.textContent = "Download";
    card.append(label, dl);
    (a.parentElement?.childNodes.length === 1 ? a.parentElement : a).replaceWith(card);
  }
}

/** Clicks on the Copy and Download buttons of a picture. */
export async function onMediaAction(el: HTMLElement): Promise<void> {
  const src = el.dataset.src ?? "";
  if (el.dataset.action === "media-download") {
    const res = await fetch(src);
    const blob = await res.blob();
    const ext = src.slice(src.lastIndexOf(".") + 1);
    const base = (el.dataset.name ?? "image").replace(/[^\w. -]+/g, "_").slice(0, 60) || "image";
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${base}.${ext}`;
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    return;
  }
  const label = el.textContent;
  try {
    const blob = await (await fetch(src)).blob();
    await navigator.clipboard.write([new ClipboardItem({ [blob.type]: blob })]);
    el.textContent = "Copied";
  } catch {
    el.textContent = "Not copied";
  }
  setTimeout(() => { el.textContent = label; }, 1500);
}

