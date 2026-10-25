import test from "node:test";
import assert from "node:assert/strict";
import { build } from "esbuild";

// ARTIFACTS: what the side pane shows and how, core/artifact.ts.
const out = await build({
  entryPoints: [new URL("../src/core/artifact.ts", import.meta.url).pathname],
  bundle: true, format: "esm", platform: "node", write: false, logLevel: "silent",
});
const { viewOf, langOfPath, isLong, lineCount, previewDoc, fileName, LONG_LINES } = await import(`data:text/javascript;base64,${Buffer.from(out.outputFiles[0].text).toString("base64")}`);

test("HTML and SVG preview in the frame, Markdown renders, the rest is code", () => {
  for (const l of ["html", "HTML", "htm", "svg"]) assert.equal(viewOf(l), "html");
  for (const l of ["md", "markdown"]) assert.equal(viewOf(l), "markdown");
  for (const l of ["ts", "rust", "text", "xml", ""]) assert.equal(viewOf(l), "code");
});

test("a file's language is its extension", () => {
  assert.equal(langOfPath("/home/kees/src/battle.ts"), "ts");
  assert.equal(langOfPath("C:\\x\\Index.HTML"), "html");
  assert.equal(langOfPath("Makefile"), "text");
});

test("long means twenty lines or more; a final newline is not a line", () => {
  assert.equal(LONG_LINES, 20);
  assert.equal(lineCount(""), 0);
  assert.equal(lineCount("a\nb\n"), 2);
  assert.equal(isLong("x\n".repeat(19)), false);
  assert.equal(isLong("x\n".repeat(20)), true);
});

test("SVG is wrapped in a page, HTML goes as written", () => {
  assert.equal(previewDoc({ title: "html", lang: "html", text: "<p>hi</p>" }), "<p>hi</p>");
  const svg = previewDoc({ title: "svg", lang: "svg", text: "<svg></svg>" });
  assert.match(svg, /^<!doctype html>/);
  assert.match(svg, /<body><svg><\/svg><\/body>/);
});

test("downloads keep a file's own name, else artifact.<ext>", () => {
  assert.equal(fileName({ title: "/home/kees/src/battle.ts", lang: "ts", text: "" }), "battle.ts");
  assert.equal(fileName({ title: "html", lang: "html", text: "" }), "artifact.html");
  assert.equal(fileName({ title: "markdown", lang: "markdown", text: "" }), "artifact.md");
  assert.equal(fileName({ title: "text", lang: "text", text: "" }), "artifact.txt");
  assert.equal(fileName({ title: "x", lang: "../../etc", text: "" }), "artifact.txt");
});
