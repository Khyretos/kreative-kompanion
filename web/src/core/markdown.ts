// Chat markdown: `marked` turns text into HTML, DOMPurify removes anything
// unsafe, and the result is returned as a DOM fragment. Model output is
// untrusted: an XSS in an agent UI is remote code execution on your PCs
// (see OpenCode CVE-2026-22813, Open WebUI CVE-2025-46719).
import { Marked } from "marked";
import DOMPurify from "dompurify";
import { enhanceCodeBlocks } from "./codeblocks";
import { enhanceMedia, FILE_HREF } from "./media";

const md = new Marked({ gfm: true, breaks: true, async: false });
// PASTE-LOG (Kees 10-27): chat text follows Discord here. Indented lines are not code (a pasted
// traceback indents every other line), and "~~~" does not open a code block (Python marks the
// failing spot with ~~~^^^ lines). Code blocks are ``` fences only.
md.use({ tokenizer: {
  code: () => undefined,
  fences(src: string) { return src.startsWith("~") ? undefined : false; },
} as never });

DOMPurify.addHook("uponSanitizeAttribute", (node, data) => {
  // Keep only "language-xyz" classes on code elements (for highlighting).
  if (data.attrName === "class") {
    data.keepAttr = node.tagName === "CODE" ? /^language-[\w+-]+$/.test(data.attrValue) :
      node.tagName === "SPAN" && /^(hljs-[\w-]+\s*)+$/.test(data.attrValue);
    data.forceKeepAttr = data.keepAttr;
  }
});

DOMPurify.addHook("afterSanitizeAttributes", (node) => {
  if (node.tagName === "IMG") {
    const src = node.getAttribute("src") ?? "";
    if (!/^\/api\/chats\/[\w-]+\/files\/[0-9a-f-]{36}\.(png|jpg|webp)$/.test(src)) {
      node.remove();
      return;
    }
    node.setAttribute("loading", "lazy");
    return;
  }
  if (node.tagName === "A") {
    const href = node.getAttribute("href") ?? "";
    // Keep in-app hash links (#task=... or #chat=...) without target/rel; block everything else.
    // CHAT-05: a file a chat tool made (a .blend scene) stays a same-site link; enhanceMedia turns it into a download card.
    if (/^#(task|chat)=[\w-]+$/.test(href) || /^#src-\d+$/.test(href) || FILE_HREF.test(href)) {
      return;
    }
    if (!/^https?:\/\//i.test(href)) node.removeAttribute("href");
    node.setAttribute("target", "_blank");
    node.setAttribute("rel", "noopener noreferrer nofollow");
  }
});

const config = {
  ALLOWED_TAGS: ["p", "br", "strong", "em", "del", "code", "pre", "ul", "ol", "li", "blockquote",
    "h1", "h2", "h3", "h4", "h5", "h6", "a", "table", "thead", "tbody", "tr", "th", "td", "hr", "img"],
  ALLOWED_ATTR: ["href", "target", "rel", "align", "src", "alt"],
  RETURN_DOM_FRAGMENT: true as const,
};

/** PASTE-LOG: leading spaces of plain lines stay (a traceback's indentation), as in Discord.
 * List items and lines inside ``` fences are left alone. */
export function keepIndent(src: string): string {
  let fenced = false;
  return src.split("\n").map((line) => {
    if (/^\s*```/.test(line)) { fenced = !fenced; return line; }
    if (fenced) return line;
    const m = /^( +)(?![-*+] |\d+[.)] )(\S.*)$/.exec(line);
    return m ? "\u00a0".repeat(m[1].length) + m[2] : line;
  }).join("\n");
}

/** Untrusted markdown in, safe DOM fragment out. */
export function renderMarkdown(src: string): DocumentFragment {
  const raw = md.parse(keepIndent(src)) as string;
  const frag = DOMPurify.sanitize(raw, config);
  enhanceCodeBlocks(frag);
  enhanceMedia(frag);
  return frag;
}
