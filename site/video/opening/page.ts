// Draws one global frame of the opening onto the canvas.
import { boxes, cell, ground } from "./draw.ts";
import { END, FIRST, HEIGHT, WIDTH, flip, flips, pieces, view } from "./plan.ts";

const DARK = "#252525";
const LIGHT = "#fbfbfa";

const canvas = document.querySelector<HTMLCanvasElement>("#frame");
const ctx = canvas?.getContext("2d");
if (!canvas || !ctx) throw new Error("index.html lacks #frame");

/** Field cells flip light ring by ring until bar 5. */
function field(f: number, v: ReturnType<typeof view>) {
  const done = f >= flips[flips.length - 1];
  ctx!.fillStyle = done ? LIGHT : DARK;
  ctx!.fillRect(0, 0, WIDTH, HEIGHT);
  if (done) return;
  const corners = [[0, 0], [WIDTH, 0], [0, HEIGHT], [WIDTH, HEIGHT]].map(([x, y]) => ground(v, x, y));
  const xs = corners.map(([x]) => x);
  const ys = corners.map(([, y]) => y);
  for (let i = Math.floor(Math.min(...xs)) - 1; i <= Math.ceil(Math.max(...xs)); i++)
    for (let j = Math.floor(Math.min(...ys)) - 1; j <= Math.ceil(Math.max(...ys)); j++) {
      const u = flip(f, i, j);
      if (u > 0) cell(ctx!, v, i, j, u, LIGHT);
    }
}

/** Renders global frame `f`. */
function render(f: number): void {
  if (f < FIRST || f >= END) throw new Error(`frame ${f} is outside the opening`);
  const v = view(f);
  field(f, v);
  boxes(ctx!, v, pieces(f, v));
}

declare global {
  interface Window {
    opening: { first: number; end: number; render: typeof render };
  }
}
window.opening = { first: FIRST, end: END, render };
