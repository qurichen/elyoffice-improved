// A desk lamp that turns the page dark and back.
import "./night.css";
import { block, stage } from "./iso.ts";
import { TEXT, type Lang } from "./text.ts";

const SIXTEENTH = 60 / 112 / 4;
/** Wave steps; the last block lands by 1.14 s. */
const STEPS = 7;
const LAND = 0.2;
const KEY = "ely-theme";
const PAGE = { light: "#fbfbfa", dark: "#252525" };

type Theme = keyof typeof PAGE;

/** Light tone to night tone, by the same three-face rule. */
export const NIGHT: Record<string, string> = {
  "#f4f4f2": "#5c5c5a",
  "#e6e6e2": "#3a3a3a",
  "#3a3a3c": "#303032",
  "#3d7fe5": "#3d7fe5",
};
const GLOW = "#f2e6c9";
const DESK = "#e6e6e2";
const PAPER = "#f4f4f2";

/** Gives every block under `root` its night tone. */
function tone(root: ParentNode): void {
  const blocks = [...root.querySelectorAll<SVGGElement>("g.block")];
  if (root instanceof SVGGElement && root.matches("g.block")) blocks.push(root);
  for (const g of blocks) {
    if (g.style.getPropertyValue("--tone-dark")) continue;
    const light = g.style.getPropertyValue("--tone").trim().toLowerCase();
    const dark = NIGHT[light];
    if (!dark) throw new Error(`no night tone for ${light}`);
    g.style.setProperty("--tone-dark", dark);
  }
}

function stored(): Theme | null {
  try {
    const v = localStorage.getItem(KEY);
    return v === "dark" || v === "light" ? v : null;
  } catch {
    return null;
  }
}

function keep(theme: Theme): void {
  try {
    localStorage.setItem(KEY, theme);
  } catch {
    // Storage may be off; the choice lasts this page.
  }
}

/** Base, post, arm, shade, bulb and a pool of light. */
function lamp(): SVGSVGElement {
  const glow = [
    block({ x: 0.9, y: 0.1, z: 0.2, w: 0.8, d: 0.8, h: 0.04, tone: DESK }),
    block({ x: 1.1, y: 0.3, z: 0.95, w: 0.4, d: 0.4, h: 0.35, tone: DESK }),
  ];
  for (const g of glow) g.style.setProperty("--tone-dark", GLOW);
  const svg = stage([
    block({ x: 0, y: 0, w: 1.8, d: 1.8, h: 0.2, tone: DESK }),
    glow[0],
    block({ x: 0.2, y: 0.35, z: 0.2, w: 0.35, d: 0.3, h: 2.0, tone: PAPER }),
    block({ x: 0.55, y: 0.35, z: 1.9, w: 0.75, d: 0.3, h: 0.3, tone: PAPER }),
    glow[1],
    block({ x: 0.95, y: 0.15, z: 1.3, w: 0.7, d: 0.7, h: 0.6, tone: PAPER }),
  ]);
  svg.setAttribute("aria-hidden", "true");
  return svg;
}

const lang = (): Lang => (document.documentElement.lang.startsWith("zh") ? "zh" : "en");

let run = 0;

/** Turns the page to `theme`, rippling out from `from`. */
function turn(theme: Theme, from: DOMRect | null): void {
  const root = document.documentElement;
  const before = root.dataset.theme;
  if (before === theme) return;
  const still = !from || !before || matchMedia("(prefers-reduced-motion: reduce)").matches;
  const id = ++run;
  root.querySelector(".night-sky")?.remove();
  if (still) {
    root.classList.remove("waving");
    root.style.removeProperty("background-color");
    root.dataset.theme = theme;
    return;
  }
  const x = from.x + from.width / 2;
  const y = from.y + from.height / 2;
  const corner = Math.max(...[0, innerWidth].flatMap((cx) => [0, innerHeight].map((cy) => Math.hypot(cx - x, cy - y))));
  const all = [...document.querySelectorAll<HTMLElement | SVGElement>("g.block, h1, h2, h3, p, a, li, button")];
  const away = all.map((el) => {
    const r = el.getBoundingClientRect();
    const seen = r.bottom > 0 && r.top < innerHeight && r.right > 0 && r.left < innerWidth;
    return { el, seen, d: Math.hypot(r.x + r.width / 2 - x, r.y + r.height / 2 - y) };
  });
  // The farthest thing in view lands on the last step.
  const reach = Math.max(1, ...away.filter((a) => a.seen).map((a) => a.d));
  for (const { el, d } of away) {
    const step = Math.min(STEPS, Math.round((d / reach) * STEPS));
    el.style.setProperty("--wave", `${(step * SIXTEENTH).toFixed(3)}s`);
  }
  // The new sky spreads at the wave's pace.
  const sky = document.createElement("div");
  sky.className = "night-sky";
  sky.style.background = PAGE[theme];
  root.style.backgroundColor = PAGE[before as Theme];
  document.body.append(sky);
  root.classList.add("waving");
  root.dataset.theme = theme;
  const last = STEPS * SIXTEENTH;
  const span = last + LAND;
  const circle = (r: number) => ({ clipPath: `circle(${r.toFixed(1)}px at ${x}px ${y}px)` });
  const grown = sky.animate(
    [circle(0), { ...circle(Math.min(reach, corner)), offset: last / span }, circle(corner)],
    { duration: span * 1000, easing: "linear", fill: "forwards" },
  );
  grown.finished.then(() => {
    if (id !== run) return;
    sky.remove();
    root.style.removeProperty("background-color");
    root.classList.remove("waving");
  });
}

/** The lamp button; it follows the system until pressed. */
export function night(): HTMLElement {
  const button = document.createElement("button");
  button.className = "night";
  const name = document.createElement("span");
  name.className = "night-name";
  button.append(lamp(), name);

  const label = () => {
    const key = document.documentElement.dataset.theme === "dark" ? "light" : "dark";
    name.dataset.t = key;
    name.textContent = TEXT[lang()][key];
  };
  const system = matchMedia("(prefers-color-scheme: dark)");
  const set = (theme: Theme, from: DOMRect | null) => {
    turn(theme, from);
    label();
  };

  tone(document);
  new MutationObserver((changes) => {
    for (const c of changes) for (const n of c.addedNodes) if (n instanceof Element) tone(n);
  }).observe(document.documentElement, { childList: true, subtree: true });

  set(stored() ?? (system.matches ? "dark" : "light"), null);
  system.addEventListener("change", () => stored() || set(system.matches ? "dark" : "light", button.getBoundingClientRect()));
  button.addEventListener("click", () => {
    const theme = document.documentElement.dataset.theme === "dark" ? "light" : "dark";
    keep(theme);
    set(theme, button.getBoundingClientRect());
  });
  return button;
}
