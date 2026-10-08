import test from "node:test";
import assert from "node:assert/strict";
import { splitFiles, kindOf, sizeLabel } from "../src/core/attachments.ts";

test("a message without a files block is returned as it is", () => {
  assert.deepEqual(splitFiles("hello"), { text: "hello", files: [] });
});

test("the files block is cut off and parsed", () => {
  const files = [{ name: "cat.png", file: "a.png", kind: "image" }, { name: "n.md", file: "b.txt", kind: "text" }];
  const r = splitFiles("Look\n\n:::files\n" + JSON.stringify(files));
  assert.equal(r.text, "Look");
  assert.deepEqual(r.files, files);
});

test("broken or odd entries never throw and are dropped", () => {
  assert.deepEqual(splitFiles("hi\n\n:::files\nnot json"), { text: "hi\n\n:::files\nnot json", files: [] });
  const r = splitFiles('x\n\n:::files\n[{"name":1},{"name":"ok.txt","file":"c.txt","kind":"text"},{"name":"bad","file":"d","kind":"exe"}]');
  assert.equal(r.text, "x");
  assert.deepEqual(r.files, [{ name: "ok.txt", file: "c.txt", kind: "text" }]);
});

test("kindOf follows the extension", () => {
  assert.equal(kindOf("Cat.PNG"), "image");
  assert.equal(kindOf("a.jpeg"), "image");
  assert.equal(kindOf("report.pdf"), "text");
  assert.equal(kindOf("main.rs"), "text");
  assert.equal(kindOf("notes.md"), "text");
  assert.equal(kindOf("a.gif"), null);
  assert.equal(kindOf("tool.exe"), null);
  assert.equal(kindOf("noextension"), null);
});

test("sizeLabel", () => {
  assert.equal(sizeLabel(532), "532 B");
  assert.equal(sizeLabel(4300), "4.2 KB");
  assert.equal(sizeLabel(1_363_149), "1.3 MB");
  assert.equal(sizeLabel(0), "0 B");
});
