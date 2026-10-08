// Highlighted code blocks with a language label, Copy and Wrap (drafted by qwen3:14b, reviewed).
import hljs from "highlight.js/lib/core";
import rust from "highlight.js/lib/languages/rust";
import typescript from "highlight.js/lib/languages/typescript";
import javascript from "highlight.js/lib/languages/javascript";
import bash from "highlight.js/lib/languages/bash";
import json from "highlight.js/lib/languages/json";
import ini from "highlight.js/lib/languages/ini";
import yaml from "highlight.js/lib/languages/yaml";
import python from "highlight.js/lib/languages/python";
import css from "highlight.js/lib/languages/css";
import xml from "highlight.js/lib/languages/xml";
import sql from "highlight.js/lib/languages/sql";
import dockerfile from "highlight.js/lib/languages/dockerfile";
import diff from "highlight.js/lib/languages/diff";
import DOMPurify from "dompurify";

hljs.registerLanguage("rust", rust);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("bash", bash);
hljs.registerLanguage("json", json);
hljs.registerLanguage("ini", ini);
hljs.registerLanguage("yaml", yaml);
hljs.registerLanguage("python", python);
hljs.registerLanguage("css", css);
hljs.registerLanguage("xml", xml);
hljs.registerLanguage("sql", sql);
hljs.registerLanguage("dockerfile", dockerfile);
hljs.registerLanguage("diff", diff);

// Register aliases
const langAliases: Record<string, string> = {
  toml: "ini",
  html: "xml",
  sh: "bash",
  shell: "bash",
  ts: "typescript",
  js: "javascript",
  py: "python",
  yml: "yaml",
  docker: "dockerfile"
};

// File extension of a downloaded code block, by (alias-resolved) language.
const extensions: Record<string, string> = {
  rust: "rs", typescript: "ts", javascript: "js", bash: "sh", json: "json", ini: "toml", yaml: "yml",
  python: "py", css: "css", xml: "html", sql: "sql", dockerfile: "Dockerfile", diff: "diff"
};
export const extFor = (lang: string): string => extensions[lang] ?? "txt";

/** Saves text as a file through a temporary link (CHAT-05). */
export function saveText(text: string, name: string, type = "text/plain"): void {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function enhanceCodeBlocks(frag: DocumentFragment): void {
  const preElements = frag.querySelectorAll("pre > code");
  for (const code of preElements) {
    const langClass = code.className.match(/^language-([\w+-]+)$/);
    const lang = langClass ? (langAliases[langClass[1]] ?? langClass[1]) : "text";
    const text = code.textContent ?? "";
    const pre = code.parentElement;

    if (pre && hljs.getLanguage(lang)) {
      try {
        const highlighted = hljs.highlight(text, { language: lang, ignoreIllegals: true }).value;
        const safe = DOMPurify.sanitize(highlighted, {
          ALLOWED_TAGS: ["span"],
          ALLOWED_ATTR: ["class"],
          RETURN_DOM_FRAGMENT: true
        });
        code.replaceChildren(...safe.childNodes);
        code.classList.add("hljs");
      } catch (e) {
        // Ignore errors during highlighting
      }
    }

    if (pre) {
      const codeblock = document.createElement("div");
      codeblock.className = "codeblock";

      const head = document.createElement("div");
      head.className = "codeblock-head";

      const langSpan = document.createElement("span");
      langSpan.className = "codeblock-lang";
      langSpan.textContent = lang === "text" ? "text" : lang;

      const wrapBtn = document.createElement("button");
      wrapBtn.type = "button";
      wrapBtn.className = "btn small";
      wrapBtn.setAttribute("data-action", "code-wrap");
      wrapBtn.setAttribute("aria-pressed", "false");
      wrapBtn.textContent = "Wrap";

      const copyBtn = document.createElement("button");
      copyBtn.type = "button";
      copyBtn.className = "btn small";
      copyBtn.setAttribute("data-action", "code-copy");
      copyBtn.textContent = "Copy";

      const downloadBtn = document.createElement("button");
      downloadBtn.type = "button";
      downloadBtn.className = "btn small";
      downloadBtn.setAttribute("data-action", "code-download");
      downloadBtn.setAttribute("data-ext", extFor(lang));
      downloadBtn.textContent = "Download";

      head.appendChild(langSpan);
      head.appendChild(wrapBtn);
      head.appendChild(copyBtn);
      head.appendChild(downloadBtn);

      // Put the wrapper where the pre is, then move the pre inside it.
      pre.parentNode?.insertBefore(codeblock, pre);
      codeblock.appendChild(head);
      codeblock.appendChild(pre);
    }
  }
}

export function onCodeAction(el: HTMLElement): void {
  const action = el.getAttribute("data-action");
  const codeblock = el.closest(".codeblock");

  if (action === "code-copy") {
    const code = codeblock?.querySelector("code");
    if (code) {
      void navigator.clipboard.writeText(code.textContent ?? "").catch(() => {});
      el.textContent = "Copied";
      setTimeout(() => {
        el.textContent = "Copy";
      }, 1500);
    }
  } else if (action === "code-download") {
    const code = codeblock?.querySelector("code");
    const ext = el.getAttribute("data-ext") ?? "txt";
    if (code) saveText(code.textContent ?? "", ext === "Dockerfile" ? ext : `code.${ext}`);
  } else if (action === "code-wrap") {
    if (codeblock) {
      codeblock.classList.toggle("wrap");
      el.setAttribute("aria-pressed", codeblock.classList.contains("wrap") ? "true" : "false");
    }
  }
}


/** CHAT-06: code colour themes. "kompas" is the default (follows light/dark); the rest are fixed palettes in styles.css. */
export const CODE_THEMES: Record<string, string> = {
  kompas: "Kreative Kompas (default)",
  dracula: "Dracula",
  "solarized-dark": "Solarized dark",
  "github-light": "GitHub light",
  "high-contrast": "High contrast",
};
const CODE_THEME_KEY = "kompanion-code-theme";

export function getCodeTheme(): string {
  try { const v = localStorage.getItem(CODE_THEME_KEY) ?? ""; return v in CODE_THEMES ? v : "kompas"; } catch { return "kompas"; }
}

export function setCodeTheme(name: string): void {
  const theme = name in CODE_THEMES ? name : "kompas";
  try { localStorage.setItem(CODE_THEME_KEY, theme); } catch { /* private mode: applies for this page only */ }
  applyCodeTheme(theme);
}

export function applyCodeTheme(name = getCodeTheme()): void {
  if (name === "kompas") delete document.documentElement.dataset.codeTheme;
  else document.documentElement.dataset.codeTheme = name;
}
