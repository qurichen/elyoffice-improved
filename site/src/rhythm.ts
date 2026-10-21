// A desk speaker; the buildings dance to its music.
import "./rhythm.css";
import { block, order, project, UNIT, type Block } from "./iso.ts";
import { ACCENT, model } from "./model.ts";
import { NIGHT } from "./night.ts";
import { slab } from "./tour.ts";
import { TEXT, type Key } from "./text.ts";

type Box = Required<Block>;

const SVG = "http://www.w3.org/2000/svg";
const BEAT = 60 / 112;
const STEP = BEAT / 4;
/** Takeoff on the beat, landing on the eighth. */
const HOP = 2 * STEP;
const COS = Math.cos(Math.PI / 6);
const SIN = Math.sin(Math.PI / 6);
const DESK = model[0].blocks[0];
const TOP = DESK.z + DESK.h;
const APPS = model.slice(1, -1).map((p) => p.name);
const ROADS = model[model.length - 1].blocks;
/** Every box the stage draws: the model and the screens. */
const DRAWN = [...model.flatMap((p) => p.blocks), ...APPS.flatMap(slab)];

/** Cabinet with two cones on its front-left face. */
const CABINET: Box = { x: 13, y: 8, z: TOP, w: 1.4, d: 1.2, h: 1.9, tone: "#3a3a3c" };
const CONE = "#e6e6e2";
const cone = (z: number, s: number, out = 0): Box => ({
  x: CABINET.x + (CABINET.w - s) / 2, y: CABINET.y + CABINET.d + out, z: TOP + z, w: s, d: 0.14, h: s, tone: CONE,
});
const WOOFER = cone(0.25, 0.9);
const CAP = cone(0.5, 0.4, 0.14);
const TWEETER = cone(1.35, 0.4);
/** How far the woofer moves at full level, in cells. */
const THROW = 0.2;

/** Cuts a road into `n` cells along one axis. */
function cut(road: Box, n: number, axis: "x" | "y"): Box[] {
  const long = axis === "x" ? road.w : road.d;
  return Array.from({ length: n }, (_, i) => {
    const at = road[axis] + (long * i) / n;
    return axis === "x" ? { ...road, x: at, w: long / n } : { ...road, y: at, d: long / n };
  });
}

/** Light runs along these cells: to ElyPDF, and to ElyTree. */
const ROUTES = [cut(ROADS[3], 11, "x"), [...cut(ROADS[4], 2, "y"), ...cut(ROADS[5], 3, "x")]];

/** Footprints a desk cell must not touch. */
const TAKEN = [...DRAWN.filter((b) => b !== DESK), CABINET, WOOFER, CAP, TWEETER];

/** Adds the speaker; the desk dances to its music. */
export function rhythm(host: HTMLElement, svg: SVGSVGElement): void {
  const specs = locate(svg);
  const buildings = APPS.map((name) => {
    const blocks = [...svg.querySelectorAll<SVGGElement>(`.block[data-part="${name}"]`)];
    if (!blocks.length) throw new Error(`${name} has no blocks; mount after touch()`);
    for (const b of blocks) b.classList.add("dance");
    return { blocks, lift: 0 };
  });
  const roads = ROUTES.map((cells) =>
    cells.map((cell) => ({ cell, g: owner(specs, ROADS.find((r) => inside(cell, r))!) })),
  );

  const speaker = [CABINET, WOOFER, CAP, TWEETER].map((spec) => {
    const g = block(spec);
    paint(g, spec.tone);
    g.classList.add("speaker");
    insert(svg, specs, g, spec);
    return g;
  });
  const woofer = speaker.slice(1, 3);
  for (const g of woofer) g.classList.add("woofer");

  const audio = document.createElement("audio");
  audio.preload = "none";
  for (const [src, type] of [["/music/ely.ogg", "audio/ogg; codecs=opus"], ["/music/ely.m4a", "audio/mp4"]]) {
    const source = document.createElement("source");
    source.src = src;
    source.type = type;
    audio.append(source);
  }

  const button = document.createElement("button");
  button.type = "button";
  button.className = "speaker-button";
  const name = document.createElement("span");
  button.append(name);
  // The button rides the stage's camera, next to it.
  const world = svg.parentElement;
  if (!world || getComputedStyle(world).position === "static") throw new Error("stage needs a positioned parent");
  host.append(audio);
  world.append(button);
  const label = (key: Key) => {
    name.dataset.t = key;
    name.textContent = TEXT[document.documentElement.lang === "zh-CN" ? "zh" : "en"][key];
  };
  label("play");

  const view = svg.viewBox.baseVal;
  const corners = [CABINET, CAP].flatMap((b) =>
    [0, 1].flatMap((i) => [0, 1].flatMap((j) => [0, 1].map((k) => project(b.x + i * b.w, b.y + j * b.d, b.z + k * b.h)))),
  );
  const [xs, ys] = [corners.map((c) => c[0]), corners.map((c) => c[1])];
  button.style.left = `${Math.min(...xs) - view.x}px`;
  button.style.top = `${Math.min(...ys) - view.y}px`;
  button.style.width = `${Math.max(...xs) - Math.min(...xs)}px`;
  button.style.height = `${Math.max(...ys) - Math.min(...ys)}px`;

  let level: { analyser: AnalyserNode; data: Float32Array<ArrayBuffer>; peak: number } | null = null;
  let context: AudioContext | null = null;
  button.addEventListener("click", async () => {
    if (!audio.paused) return audio.pause();
    if (!context) {
      context = new AudioContext();
      const analyser = context.createAnalyser();
      analyser.fftSize = 512;
      context.createMediaElementSource(audio).connect(analyser).connect(context.destination);
      level = { analyser, data: new Float32Array(analyser.fftSize), peak: 0 };
    }
    await context.resume();
    await audio.play();
  });

  const still = matchMedia("(prefers-reduced-motion: reduce)");
  let frame = 0;
  let clock = { media: -1, wall: 0 };

  const pulses = ROUTES.map(() => {
    const p = document.createElementNS(SVG, "polygon");
    p.setAttribute("class", "pulse");
    return p;
  });

  const tick = (now: number) => {
    frame = requestAnimationFrame(tick);
    // currentTime updates coarsely; extrapolate between updates.
    if (audio.currentTime !== clock.media) clock = { media: audio.currentTime, wall: now };
    const t = clock.media + (now - clock.wall) / 1000;
    const n = Math.floor(t / BEAT);
    const into = (t - n * BEAT) / HOP;
    const lift = into < 1 ? 4 * into * (1 - into) : 0;
    const bar = Math.floor(n / 4);
    const who = n % 4 === 0 ? -1 : (bar * 3 + (n % 4) - 1) % APPS.length;
    buildings.forEach((b, i) => hop(b, who < 0 || who === i ? lift : 0));
    const cell = Math.floor((t - bar * 4 * BEAT) / STEP);
    roads.forEach((route, i) => pulse(pulses[i], route[cell] ?? null));
    if (!level) throw new Error("playing without an analyser");
    level.analyser.getFloatTimeDomainData(level.data);
    const peak = level.data.reduce((m, v) => Math.max(m, Math.abs(v)), 0);
    level.peak = Math.max(peak, level.peak * 0.8);
    for (const g of woofer) push(g, Math.min(1, level.peak * 1.5) * THROW);
  };

  const run = () => {
    const go = !audio.paused && !still.matches && document.visibilityState === "visible";
    if (go && !frame) frame = requestAnimationFrame(tick);
    if (go) return;
    cancelAnimationFrame(frame);
    frame = 0;
    for (const b of buildings) hop(b, 0);
    for (const p of pulses) p.remove();
    for (const g of woofer) g.style.removeProperty("translate");
  };

  const playing = (on: boolean) => {
    host.classList.toggle("playing", on);
    label(on ? "pause" : "play");
    for (const g of [speaker[1], speaker[3]]) paint(g, on ? ACCENT : CONE);
    if (on) lay(null);
    run();
  };
  audio.addEventListener("play", () => playing(true));
  audio.addEventListener("pause", () => playing(false));
  audio.addEventListener("ended", () => playing(false));
  still.addEventListener("change", run);
  document.addEventListener("visibilitychange", run);

  let tile: { at: string; g: SVGGElement | null } = { at: "", g: null };
  const lay = (cell: [number, number] | null) => {
    const at = cell ? cell.join() : "";
    if (at === tile.at) return;
    tile.g?.remove();
    if (tile.g) specs.delete(tile.g);
    tile = { at, g: null };
    if (!cell) return;
    const spec: Box = { x: cell[0], y: cell[1], z: TOP, w: 1, d: 1, h: 0.12, tone: "#f4f4f2" };
    if (spec.x < 0 || spec.y < 0 || spec.x >= DESK.w || spec.y >= DESK.d) return;
    if (TAKEN.some((b) => b.x < spec.x + 1 && spec.x < b.x + b.w && b.y < spec.y + 1 && spec.y < b.y + b.d)) return;
    tile.g = block(spec);
    paint(tile.g, spec.tone);
    tile.g.classList.add("tile");
    insert(svg, specs, tile.g, spec);
  };
  svg.addEventListener("pointermove", (e) => {
    const ground = e.target instanceof SVGElement && e.target.dataset.part === "";
    if (e.pointerType !== "mouse" || !audio.paused || !ground) return lay(null);
    const ctm = svg.getScreenCTM();
    if (!ctm) throw new Error("stage is not rendered");
    const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(ctm.inverse());
    // Invert the projection on the desk's top plane.
    const a = p.x / (COS * UNIT);
    const b = (p.y / UNIT + TOP) / SIN;
    lay([Math.floor((a + b) / 2), Math.floor((b - a) / 2)]);
  });
  svg.addEventListener("pointerleave", () => lay(null));
}

/** Sets a block's tone and its night tone. */
function paint(g: SVGGElement, tone: string) {
  const dark = NIGHT[tone];
  if (!dark) throw new Error(`no night tone for ${tone}`);
  g.style.setProperty("--tone", tone);
  g.style.setProperty("--tone-dark", dark);
}

/** Lifts a building by `lift` cells. */
function hop(b: { blocks: SVGGElement[]; lift: number }, lift: number) {
  if (lift === b.lift) return;
  b.lift = lift;
  for (const g of b.blocks) {
    if (lift) g.style.setProperty("--hop", `${(-lift * UNIT).toFixed(1)}px`);
    else g.style.removeProperty("--hop");
  }
}

/** Lights one road cell, or none. */
function pulse(p: SVGPolygonElement, at: { cell: Box; g: SVGGElement } | null) {
  if (!at) return p.remove();
  const { x, y, w, d } = at.cell;
  const z = at.cell.z + at.cell.h;
  const points = [project(x, y, z), project(x + w, y, z), project(x + w, y + d, z), project(x, y + d, z)];
  p.setAttribute("points", points.map(([px, py]) => `${px.toFixed(2)},${py.toFixed(2)}`).join(" "));
  if (p.parentNode !== at.g) at.g.append(p);
}

/** Pushes the woofer out along its face normal. */
function push(g: SVGGElement, cells: number) {
  g.style.translate = `${(-COS * UNIT * cells).toFixed(2)}px ${(SIN * UNIT * cells).toFixed(2)}px`;
}

function inside(a: Box, b: Box): boolean {
  return a.x >= b.x && a.y >= b.y && a.x + a.w <= b.x + b.w + 1e-9 && a.y + a.d <= b.y + b.d + 1e-9;
}

/** Matches drawn blocks to model boxes by top face. */
function locate(svg: SVGSVGElement): Map<SVGGElement, Box> {
  const near = (pt: DOMPoint, [x, y]: [number, number]) => Math.abs(pt.x - x) < 0.02 && Math.abs(pt.y - y) < 0.02;
  const found = new Map<SVGGElement, Box>();
  for (const g of svg.querySelectorAll<SVGGElement>(".block")) {
    const top = g.querySelector<SVGPolygonElement>(".face.top");
    if (!top) throw new Error("block has no top");
    const spec = DRAWN.find(
      (b) =>
        near(top.points[0], project(b.x, b.y, b.z + b.h)) && near(top.points[2], project(b.x + b.w, b.y + b.d, b.z + b.h)),
    );
    if (!spec) throw new Error("stage has a block the model lacks");
    found.set(g, spec);
  }
  return found;
}

function owner(specs: Map<SVGGElement, Box>, spec: Box): SVGGElement {
  const g = [...specs].find(([, s]) => s === spec)?.[0];
  if (!g) throw new Error("road is not drawn");
  return g;
}

/** Inserts `g` after what it covers, before its coverers. */
function insert(svg: SVGSVGElement, specs: Map<SVGGElement, Box>, g: SVGGElement, spec: Box) {
  const drawn = [...svg.querySelectorAll<SVGGElement>(".block")];
  let last = -1;
  let first = drawn.length;
  drawn.forEach((b, i) => {
    const s = specs.get(b);
    if (!s) throw new Error("stage has an unknown block");
    if (order([spec, s])[0] === 1) last = i;
    else if (order([s, spec])[0] === 1) first = Math.min(first, i);
  });
  if (last >= first) throw new Error("no paint slot for the block");
  if (last >= 0) drawn[last].after(g);
  else drawn[first].before(g);
  specs.set(g, spec);
}
