/** image: shown as a picture; text: read by the model; file: any other file, only stored and downloaded. */
export type AttachedKind = "image" | "text" | "file";
export interface Attached { name: string; file: string; kind: AttachedKind }

/** The message without its files block (also while the block is still arriving), and the attachments. */
export function splitFiles(text: string): { text: string; files: Attached[] } {
  const i = text.lastIndexOf("\n\n:::files\n");
  if (i < 0) return { text, files: [] };
  try {
    const raw: unknown = JSON.parse(text.slice(i + "\n\n:::files\n".length).split("\n")[0]);
    if (!Array.isArray(raw)) return { text, files: [] };
    const files = raw.flatMap((x): Attached[] => {
      const o = x as Record<string, unknown>;
      if (typeof o?.name !== "string" || typeof o.file !== "string" || (o.kind !== "image" && o.kind !== "text" && o.kind !== "file")) return [];
      return [{ name: o.name, file: o.file, kind: o.kind }];
    });
    return { text: text.slice(0, i), files };
  } catch {
    return { text, files: [] };
  }
}

/** Returns the kind based on the extension. Any other file is a "file"; the server reads an unknown extension as text when its content is text. */
export function kindOf(name: string): AttachedKind {
  const dot = name.lastIndexOf(".");
  if (dot < 0) return "file";
  const ext = name.slice(dot + 1).toLowerCase();
  if (["png", "jpg", "jpeg", "webp"].includes(ext)) return "image";
  if (["pdf", "md", "markdown", "txt", "text", "rst", "csv", "json", "toml", "yaml", "yml", "xml", "html", "htm", "log", "ini", "cfg", "sql", "sh", "css", "rs", "py", "ts", "tsx", "js", "jsx", "gd", "c", "h", "cpp", "hpp", "java", "go", "rb", "kt", "cs"].includes(ext)) return "text";
  return "file";
}

/** The name a sent file is saved under: its own, except a PDF or web page, which is kept as the text that was read from it. */
export function downloadName(f: Attached): string {
  return f.kind === "text" && /\.(pdf|html?)$/i.test(f.name) ? f.name.replace(/\.[^.]+$/, "") + ".txt" : f.name;
}

/** Returns a human-readable size label. */
export function sizeLabel(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
