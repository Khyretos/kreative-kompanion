// CHAT-07: live markdown styling for the composer. A transparent-text textarea sits over a
// mirror that paints the same text with markdown styling, so the raw markdown stays the value
// that is sent and keyboard, IME, paste and mobile behaviour are the browser's own. Styling
// never changes glyph widths (colour, tint, underline only), or the caret would drift.
// Built with DOM nodes, not innerHTML (the app enforces Trusted Types).

type Kid = Node | string;

const span = (cls: string, ...kids: Kid[]): HTMLSpanElement => {
  const s = document.createElement("span");
  s.className = cls;
  s.append(...kids);
  return s;
};

/** Inline spans on one line: code, bold, strike, italic, links. */
function inline(line: string): Kid[] {
  const out: Kid[] = [];
  const re = /(`[^`\n]+`)|(\*\*[^*\n]+\*\*|__[^_\n]+__)|(~~[^~\n]+~~)|(\*[^*\s][^*\n]*\*|_[^_\s][^_\n]*_)|(\[[^\]\n]*\]\([^)\n]*\))/g;
  let last = 0;
  for (let m = re.exec(line); m; m = re.exec(line)) {
    out.push(line.slice(last, m.index));
    const cls = m[1] ? "lm-code" : m[2] ? "lm-b" : m[3] ? "lm-s" : m[4] ? "lm-i" : "lm-link";
    out.push(span(cls, m[0]));
    last = m.index + m[0].length;
  }
  out.push(line.slice(last));
  return out;
}

function block(line: string, inTable: boolean): Kid[] {
  let m = /^(\s{0,3}#{1,6}\s)(.*)$/.exec(line);
  if (m) return [span("lm-h", span("lm-mark", m[1]!), ...inline(m[2]!))];
  m = /^(\s*>\s?)(.*)$/.exec(line);
  if (m) return [span("lm-quote", span("lm-mark", m[1]!), ...inline(m[2]!))];
  m = /^(\s*(?:[-*+]|\d+[.)])\s(?:\[[ xX]\]\s)?)(.*)$/.exec(line);
  if (m) return [span("lm-li", m[1]!), ...inline(m[2]!)];
  if (/^\s*([-*_])(\s*\1){2,}\s*$/.test(line)) return [span("lm-mark", line)];
  if (inTable || /^\s*\|.*\|\s*$/.test(line)) {
    const kids: Kid[] = [];
    line.split("|").forEach((c, i) => {
      if (i) kids.push(span("lm-mark", "|"));
      if (/^[\s:-]+$/.test(c)) kids.push(span("lm-mark", c));
      else kids.push(...inline(c));
    });
    return [span("lm-row", ...kids)];
  }
  return inline(line);
}

/** Nodes for the mirror: the same text as `src`, newlines kept. */
export function liveMarkdown(src: string): DocumentFragment {
  const lines = src.split("\n");
  const frag = document.createDocumentFragment();
  let fence = false;
  lines.forEach((line, i) => {
    if (i) frag.append("\n");
    if (/^\s*(```|~~~)/.test(line)) { fence = !fence; frag.append(span("lm-fence", line)); return; }
    if (fence) { frag.append(span("lm-fence", line)); return; }
    const inTable = /^\s*\|/.test(line) || (i > 0 && /^\s*\|/.test(lines[i - 1] ?? "") && line.includes("|"));
    frag.append(...block(line, inTable));
  });
  // A trailing newline needs a visible last line so the mirror grows with the textarea.
  if (src.endsWith("\n")) frag.append(" ");
  return frag;
}

/** Puts a mirror behind the textarea; returns a function that re-syncs it (call after setting .value). */
export function attachLiveMarkdown(ta: HTMLTextAreaElement): () => void {
  const wrap = ta.parentElement!;
  const mirror = document.createElement("div");
  mirror.className = "lm-mirror";
  mirror.setAttribute("aria-hidden", "true");
  wrap.insertBefore(mirror, ta);
  wrap.classList.add("lm-wrap");
  const sync = () => {
    mirror.replaceChildren(liveMarkdown(ta.value));
    mirror.style.width = `${ta.clientWidth}px`; // a scrollbar narrows the text; wrap the same
    mirror.style.height = `${ta.clientHeight}px`;
    mirror.scrollTop = ta.scrollTop;
  };
  ta.addEventListener("input", sync);
  ta.addEventListener("scroll", () => { mirror.scrollTop = ta.scrollTop; });
  new ResizeObserver(sync).observe(ta);
  sync();
  return sync;
}
