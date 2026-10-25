import test from "node:test";
import assert from "node:assert/strict";
import { build } from "esbuild";

// commandcard.ts imports without file extensions, so bundle it first, then load the bundle.
const out = await build({
  entryPoints: [new URL("../src/core/commandcard.ts", import.meta.url).pathname],
  bundle: true,
  format: "esm",
  platform: "node",
  write: false,
  logLevel: "silent",
});
const { commandText, commandLang, riskOf } = await import(`data:text/javascript;base64,${Buffer.from(out.outputFiles[0].text).toString("base64")}`);

const sh = (command: string) => ({ tool: "shell", command });

test("CMD-01: the command reads as typed", () => {
  assert.equal(commandText({ tool: "shell", cwd: "/srv", command: "ls -la" }), "cd /srv && ls -la");
  assert.equal(commandText({ tool: "package", manager: "apt", action: "install", names: ["git"] }), "apt install git");
  assert.equal(commandText({ tool: "service", action: "restart", unit: "nginx" }), "systemctl restart nginx");
  assert.equal(commandText({ tool: "write_file", path: "/a", content: "abc", append: true }), "append to /a (3 characters)");
  assert.equal(commandText({ tool: "write_file", path: "/a", content: "abc", offset: 12 }), "write at byte 12 of /a (3 characters)");
  assert.equal(commandText({ tool: "write_file", path: "/a", content: "abc" }), "write /a (3 characters)");
  assert.equal(commandLang(sh("ls")), "bash");
  assert.equal(commandLang({ tool: "read_file", path: "/a" }), "text");
});

test("CMD-01: risk tags from the tool and the command", () => {
  for (const c of ["ls -la", "cd /srv && git status", "grep -rn foo src | head -5", "docker ps", "cat a > /dev/null"]) assert.equal(riskOf(sh(c)), "read", c);
  for (const c of ["rm -rf build", "cd /x && rm a", "git clean -fdx", "docker rmi foo", "find . -name '*.tmp' -delete"]) assert.equal(riskOf(sh(c)), "delete", c);
  for (const c of ["curl -s https://example.com", "git push origin main", "npm install", "apt-get install -y git"]) assert.equal(riskOf(sh(c)), "network", c);
  for (const c of ["echo hi > notes.txt", "sed -i s/a/b/ f", "mkdir out", "cargo build", "make"]) assert.equal(riskOf(sh(c)), "write", c);
  assert.equal(riskOf({ tool: "read_file", path: "/a" }), "read");
  assert.equal(riskOf({ tool: "edit_file", path: "/a" }), "write");
  assert.equal(riskOf({ tool: "package", action: "remove", names: ["x"] }), "delete");
  assert.equal(riskOf({ tool: "package", action: "install", names: ["x"] }), "network");
});

test("AGENT-SPAWN: a spawn step has no command or risk, and its result names what it made", async () => {
  const { spawnOf, spawnedRef } = await import(`data:text/javascript;base64,${Buffer.from(out.outputFiles[0].text).toString("base64")}`);
  const tool = { tool: "start_work", kind: "task", title: "Ship", prompt: "Goal: ship" };
  assert.equal(commandText(tool), "");
  assert.equal(riskOf(tool), undefined);
  assert.deepEqual(spawnOf(tool), { kind: "task", title: "Ship", prompt: "Goal: ship" });
  assert.equal(spawnOf({ tool: "shell", command: "ls" }), undefined);
  assert.deepEqual(spawnedRef('Started the chat "X" (chat=c-12ab); it is answering.'), { kind: "chat", id: "c-12ab" });
  assert.deepEqual(spawnedRef("Added the task \"Y\" to this project (task=t9)."), { kind: "task", id: "t9" });
  assert.equal(spawnedRef("failed: no"), undefined);
});
