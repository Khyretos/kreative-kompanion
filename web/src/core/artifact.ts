// ARTIFACTS: what the side pane can show. Pure helpers (tested in tests-node/artifact.test.mts).

export interface Artifact {
  /** A file path, or the code block's language. */
  title: string;
  /** The language as written ("html", "md", "ts"…), not the highlighter's alias. */
  lang: string;
  text: string;
}

export type ArtifactView = "html" | "markdown" | "code";

/** How the pane shows it: HTML and SVG run in the sandboxed frame, Markdown renders, the rest is code. */
export function viewOf(lang: string): ArtifactView {
  const l = lang.toLowerCase();
  if (l === "html" || l === "htm" || l === "svg") return "html";
  if (l === "md" || l === "markdown") return "markdown";
  return "code";
}

/** The language of a file by its extension, for files the model writes or reads. */
export function langOfPath(path: string): string {
  const m = /\.([\w-]+)$/.exec(path.toLowerCase());
  return m ? m[1] : "text";
}

/** A code block this long shows a few lines in the chat and opens in the pane. */
export const LONG_LINES = 20;
export const lineCount = (text: string): number => (text === "" ? 0 : text.replace(/\n$/, "").split("\n").length);
export const isLong = (text: string): boolean => lineCount(text) >= LONG_LINES;

/** The document the preview frame gets: SVG alone is wrapped so it shows at its own size. */
export function previewDoc(a: Artifact): string {
  return a.lang.toLowerCase() === "svg"
    ? `<!doctype html><html><head><meta charset="utf-8"><style>body{margin:0;display:grid;place-items:center;min-height:100vh}</style></head><body>${a.text}</body></html>`
    : a.text;
}

/** The download name: the file's own name, else "artifact.<ext>". */
export function fileName(a: Artifact): string {
  const base = a.title.split(/[\\/]/).pop() ?? "";
  if (/\.[\w-]+$/.test(base)) return base;
  const ext: Record<string, string> = { markdown: "md", typescript: "ts", javascript: "js", python: "py", rust: "rs", bash: "sh", shell: "sh", yaml: "yml", text: "txt" };
  const l = a.lang.toLowerCase();
  return `artifact.${ext[l] ?? (/^[a-z0-9]{1,6}$/.test(l) ? l : "txt")}`;
}
