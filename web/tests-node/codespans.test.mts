import test from "node:test";
import assert from "node:assert/strict";
import { build } from "esbuild";

// AUDIT: step summaries such as "Read `Cargo.toml`" show the backticked part as code, core/html.ts.
const out = await build({
  entryPoints: [new URL("../src/core/html.ts", import.meta.url).pathname],
  bundle: true, format: "esm", platform: "node", write: false, logLevel: "silent",
});
const { codeSpans } = await import(`data:text/javascript;base64,${Buffer.from(out.outputFiles[0].text).toString("base64")}`);

test("backticked parts become code, the rest is escaped text", () => {
  assert.equal(codeSpans("Read `Cargo.toml`").value, "Read <code>Cargo.toml</code>");
  assert.equal(codeSpans("Run `a < b` & `c`").value, "Run <code>a &lt; b</code> &amp; <code>c</code>");
  assert.equal(codeSpans("no code here").value, "no code here");
  assert.equal(codeSpans("one ` stray").value, "one ` stray");
  assert.equal(codeSpans("`<script>`").value, "<code>&lt;script&gt;</code>");
});
