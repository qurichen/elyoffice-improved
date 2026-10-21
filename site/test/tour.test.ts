import { test } from "node:test";
import assert from "node:assert/strict";
import { order, type Block } from "../src/iso.ts";
import { model } from "../src/model.ts";
import { slab } from "../src/tour.ts";

type Box = Required<Block>;
const apps = model.slice(1, -1);
const all = model.flatMap((p) => p.blocks);
const screens = apps.map((p) => slab(p.name));

const spans = (b: Box) => [
  [b.x - b.z - b.h, b.x + b.w - b.z],
  [b.y - b.z - b.h, b.y + b.d - b.z],
  [b.x - b.y - b.d, b.x + b.w - b.y],
];
const overlap = (a: Box, b: Box) => spans(a).every(([lo, hi], i) => lo < spans(b)[i][1] && spans(b)[i][0] < hi);
const drawn = (list: Box[]) => {
  const at = order(list);
  return (b: Box) => at.indexOf(list.indexOf(b));
};

test("the desk and every screen draw without intersections", () => {
  const list = [...all, ...screens.flat()];
  assert.equal(order(list).length, list.length);
});

test("nothing covers a screen's face, so its clip shows true", () => {
  for (const [i, p] of apps.entries()) {
    const rows = screens[i].filter((b) => b.w > 0.2);
    const list = [...all, ...screens[i]];
    const rank = drawn(list);
    for (const row of rows)
      for (const b of all) assert.ok(!(overlap(row, b) && rank(b) > rank(row)), `${p.name} covered by ${JSON.stringify(b)}`);
  }
});

test("screens are sixteen by nine", () => {
  for (const rows of screens) {
    const face = rows.filter((b) => b.w > 0.2);
    const h = face.reduce((s, b) => s + b.h, 0);
    assert.ok(Math.abs(face[0].w / h - 16 / 9) < 1e-9);
  }
});
