import test from "node:test";
import assert from "node:assert/strict";
import { build } from "esbuild";

// COMPACT-UI: how alive a running task is, core/liveness.ts.
const out = await build({
  entryPoints: [new URL("../src/core/liveness.ts", import.meta.url).pathname],
  bundle: true, format: "esm", platform: "node", write: false, logLevel: "silent",
});
const { liveness, shortAge } = await import(`data:text/javascript;base64,${Buffer.from(out.outputFiles[0].text).toString("base64")}`);

const now = Date.parse("2026-10-25T12:00:00Z");
const ago = (s: number) => new Date(now - s * 1000).toISOString();

test("live within 30 s, quiet after, stalled after 10 min, board without a Kompanion run", () => {
  assert.equal(liveness({ activeRun: true, lastActivity: ago(5) }, now).kind, "live");
  assert.equal(liveness({ activeRun: true, lastActivity: ago(31) }, now).kind, "quiet");
  assert.equal(liveness({ activeRun: true, lastActivity: ago(599) }, now).kind, "quiet");
  assert.equal(liveness({ activeRun: true, lastActivity: ago(600) }, now).kind, "stalled");
  assert.equal(liveness({ activeRun: true, lastActivity: null }, now).kind, "stalled");
  const board = liveness({ activeRun: false, lastActivity: null, updatedAt: ago(21 * 86_400) }, now);
  assert.deepEqual([board.kind, shortAge(board.age)], ["board", "21 d"]);
});

test("ages read short", () => {
  assert.deepEqual([shortAge(40), shortAge(720), shortAge(3 * 3600), shortAge(2 * 86_400)], ["40 s", "12 min", "3 h", "2 d"]);
});
