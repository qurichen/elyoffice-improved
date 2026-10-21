// Three props: offline, open source, local files.
import { block, stage } from "./iso.ts";
import { grow } from "./beat.ts";
import { said, type Key } from "./text.ts";

const PAPER = "#f4f4f2";
const DESK = "#e6e6e2";

const t = (x: number, y: number, z: number, w: number, d: number, h: number, tone = PAPER) =>
  block({ x, y, z, w, d, h, tone });

/** Offline: a plug pulled from its socket. */
const cable = () => [
  t(0, 0, 0, 7, 3, 0.3, DESK),
  t(0.4, 1.3, 0.3, 2.6, 0.4, 0.4),
  t(3, 1.15, 0.3, 0.9, 0.7, 0.7),
  t(5, 0.9, 0.3, 1.4, 1.2, 1.2),
];

/** Open source: a box with its lid laid open. */
const box = () => [
  t(0, 0, 0, 7, 4, 0.3, DESK),
  t(1, 0.6, 0.3, 3, 0.2, 1.6),
  t(1, 0.8, 0.3, 0.2, 2.2, 1.6),
  t(3.8, 0.8, 0.3, 0.2, 2.2, 1.6),
  t(1.2, 0.8, 0.3, 2.6, 2.2, 0.2),
  t(1, 3, 0.3, 3, 0.2, 1.6),
  t(4.3, 0.6, 0.3, 2.2, 2.6, 0.12),
];

/** Local files: a folder holding one sheet. */
const folder = () => [
  t(0, 0, 0, 7, 3, 0.3, DESK),
  t(1, 1, 0.3, 4.6, 0.2, 2.6),
  t(1, 1, 2.9, 1.6, 0.2, 0.4),
  t(1.4, 1.2, 0.3, 3.8, 0.1, 2.3, "#3d7fe5"),
  t(1, 1.3, 0.3, 4.6, 0.2, 2),
];

/** A prop with its line. */
function prop(blocks: SVGGElement[], line: Key): HTMLElement {
  grow(blocks.map((b) => [b]));
  const svg = stage(blocks);
  svg.classList.add("grow");
  svg.setAttribute("aria-hidden", "true");
  const p = said("p", line);
  const cell = document.createElement("div");
  cell.className = "prop";
  cell.append(svg, p);
  return cell;
}

/** The props row. */
export function props(): HTMLElement {
  const row = document.createElement("section");
  row.className = "props";
  row.append(prop(cable(), "offline"), prop(box(), "open"), prop(folder(), "local"));
  return row;
}

/** Starts each model's growth when it scrolls near. */
export function reveal(sections: HTMLElement[]): void {
  const seen = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue;
        for (const svg of e.target.querySelectorAll(".stage")) svg.classList.add("seen");
        seen.unobserve(e.target);
      }
    },
    { rootMargin: "0px 0px -20% 0px" },
  );
  for (const s of sections) seen.observe(s);
}
