// CHAT-04: the sources under an answer. The server ends a saved answer with a `:::sources` line
// of JSON (name, url, excerpt per source); the app shows it as an expandable list. In the text,
// web sources are ordinary links and knowledge sources are `[name](src:n)` links that open the list
// at that source. Everything is built with textContent: model and web text stays untrusted.
export interface Source { n: number; name: string; url: string | null; excerpt: string }

const BLOCK = /^:::sources[ \t]*\r?\n([^\n]*)/;

/** The answer without its sources block (also while the block is still arriving), and the sources. */
export function splitSources(text: string): { text: string; sources: Source[] } {
  const i = text.lastIndexOf(":::sources");
  if (i < 0) return { text, sources: [] };
  const m = BLOCK.exec(text.slice(i));
  const cut = text.slice(0, i).replace(/\n+$/, "");
  if (!m) return { text: cut, sources: [] };
  try {
    const raw: unknown = JSON.parse(m[1]);
    const list = Array.isArray(raw) ? raw : [];
    const sources = list.flatMap((x): Source[] => {
      const o = x as Record<string, unknown>;
      if (typeof o?.n !== "number" || typeof o.name !== "string") return [];
      const url = typeof o.url === "string" && /^https?:\/\//i.test(o.url) ? o.url : null;
      return [{ n: o.n, name: o.name, url, excerpt: typeof o.excerpt === "string" ? o.excerpt : "" }];
    });
    return { text: cut, sources };
  } catch {
    return { text: cut, sources: [] };
  }
}

/** `[name](src:3)` becomes an in-app link the markdown sanitiser keeps. */
export const linkSources = (text: string): string => text.replace(/\]\(src:(\d+)\)/g, "](#src-$1)");

const host = (u: string): string => { try { return new URL(u).hostname.replace(/^www\./, ""); } catch { return u; } };

export function renderSources(sources: Source[]): HTMLElement {
  const box = document.createElement("details");
  box.className = "sources";
  const sum = document.createElement("summary");
  sum.textContent = `Sources (${sources.length})`;
  const ol = document.createElement("ol");
  for (const s of sources) {
    const li = document.createElement("li");
    li.dataset.n = String(s.n);
    li.id = `src-${s.n}`;
    const head = document.createElement("div");
    head.className = "src-head";
    if (s.url) {
      const a = document.createElement("a");
      a.href = s.url;
      a.target = "_blank";
      a.rel = "noopener noreferrer nofollow";
      a.textContent = s.name;
      const h = document.createElement("span");
      h.className = "muted small";
      h.textContent = host(s.url);
      head.append(a, h);
    } else {
      head.textContent = s.name;
    }
    li.append(head);
    if (s.excerpt) {
      const q = document.createElement("blockquote");
      q.className = "src-excerpt";
      q.textContent = s.excerpt;
      li.append(q);
    }
    ol.append(li);
  }
  box.append(sum, ol);
  return box;
}

// A click on a knowledge citation opens the list of its own message and marks that source.
document.addEventListener("click", (e) => {
  const a = (e.target as Element | null)?.closest?.('a[href^="#src-"]');
  if (!a) return;
  e.preventDefault();
  const body = a.closest(".body");
  const box = body?.querySelector<HTMLDetailsElement>("details.sources");
  const li = box?.querySelector<HTMLElement>(`li[data-n="${a.getAttribute("href")!.slice(5)}"]`);
  if (!box || !li) return;
  box.open = true;
  box.querySelectorAll(".hit").forEach((x) => x.classList.remove("hit"));
  li.classList.add("hit");
  li.scrollIntoView({ block: "nearest" });
});
