import test from "node:test";
import assert from "node:assert/strict";
import { build } from "esbuild";

// studio.ts imports without file extensions, so bundle it first, then load the bundle.
const out = await build({
  entryPoints: [new URL("../src/views/studio.ts", import.meta.url).pathname],
  bundle: true,
  format: "esm",
  platform: "node",
  write: false,
  logLevel: "silent",
});
const code = out.outputFiles[0].text;
const { runCard } = (await import("data:text/javascript;base64," + Buffer.from(code).toString("base64"))) as {
  runCard: (r: unknown, typeLabel: string, projects: unknown[], sentTo: string[], picked?: boolean) => { toString(): string };
};

function card(files: string[]): string {
  const run = {
    id: "r1", type: "image-to-3d", gpu: "g", prompt: "a cube", size: "square", state: "done", error: null,
    files, startedAt: "2026-01-01T00:00:00Z", endedAt: "2026-01-01T00:01:00Z",
  };
  return runCard(run, "3D model", [], []).toString();
}

test("a 3D run with a .glb shows View in 3D and the download", () => {
  const h = card(["/f/model.glb", "/f/preview.png"]);
  assert.match(h, /data-action="studio-3d"/);
  assert.match(h, /data-src="\/f\/model\.glb"/);
  assert.match(h, /Download \.glb/);
  assert.match(h, /studio-3d-preview/);
});

test("a 3D run without a .glb shows no 3D buttons", () => {
  const h = card([]);
  assert.doesNotMatch(h, /data-action="studio-3d"/);
  assert.doesNotMatch(h, /Download \.glb/);
  assert.doesNotMatch(h, /download=/);
});
