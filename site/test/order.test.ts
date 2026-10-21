import { test } from "node:test";
import assert from "node:assert/strict";
import { order } from "../src/iso.ts";

const box = (x: number, y: number, z: number, w: number, d: number, h: number) => ({ x, y, z, w, d, h, tone: "" });
const desk = box(0, 0, 0, 12, 12, 0.5);

test("blocks apart on screen add no constraint, so no false cycle", () => {
  const list = [desk, box(0, 2, 0.5, 1, 1, 1), box(1, 0, 0.5, 2, 1, 1), box(0, 1, 0.5, 2, 1, 1)];
  const drawn = order(list);
  assert.equal(drawn[0], 0, "the desk is drawn first");
  assert.ok(drawn.indexOf(3) < drawn.indexOf(1));
});

test("the desk scene draws ground, then blocks", () => {
  const list = [desk, box(2, 2, 0.5, 3, 3, 4), box(7, 2, 0.5, 3, 3, 2), box(2, 7, 0.5, 3, 3, 1)];
  assert.equal(order(list)[0], 0);
});

test("a block in front draws after the one it covers", () => {
  const back = box(0, 0, 0, 2, 2, 4);
  const front = box(2, 2, 0, 2, 2, 1);
  assert.deepEqual(order([front, back]), [1, 0]);
});

test("intersecting blocks fail loudly", () => {
  assert.throws(() => order([box(0, 0, 0, 2, 2, 2), box(1, 1, 1, 2, 2, 2)]), /intersect/);
});
