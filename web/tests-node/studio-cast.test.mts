import test from "node:test";
import assert from "node:assert/strict";
import { CAST_MIN, CAST_MAX, castNew, castAdd, castRemove, castMove, castType, castParams, castProblem } from "../src/views/studio-cast.ts";

test("a new cast has the minimum number of empty people", () => {
  assert.deepEqual(castNew(), ["", ""]);
  assert.equal(CAST_MIN, 2);
  assert.equal(CAST_MAX, 4);
});

test("people can be added up to the maximum and removed down to the minimum", () => {
  let p = castNew();
  p = castAdd(p);
  p = castAdd(p);
  assert.equal(p.length, 4);
  assert.equal(castAdd(p).length, 4);
  assert.deepEqual(castRemove(["a", "b", "c", "d"], 1), ["a", "c", "d"]);
  assert.deepEqual(castRemove(["a", "b"], 0), ["a", "b"]);
});

test("moving a person swaps places with the neighbour and never leaves the row", () => {
  assert.deepEqual(castMove(["a", "b", "c"], 0, 1), ["b", "a", "c"]);
  assert.deepEqual(castMove(["a", "b", "c"], 2, -1), ["a", "c", "b"]);
  assert.deepEqual(castMove(["a", "b", "c"], 0, -1), ["a", "b", "c"]);
  assert.deepEqual(castMove(["a", "b", "c"], 2, 1), ["a", "b", "c"]);
});

test("the workflow type and the person parameters follow the cast", () => {
  assert.equal(castType(3), "vn-cast-3");
  assert.deepEqual(castParams([" a girl ", "a boy"]), { person1: "a girl", person2: "a boy" });
});

test("an incomplete cast says what is missing, a complete one says nothing", () => {
  assert.equal(castProblem("", ["a", "b"]), "Describe the place and mood.");
  assert.equal(castProblem("roof", ["a", " "]), "Describe person 2.");
  assert.equal(castProblem("roof", ["", "b"]), "Describe person 1.");
  assert.equal(castProblem("roof", ["a", "b"]), "");
});
