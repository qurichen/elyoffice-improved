// The film on a block screen; it flattens to play.
import { block, project, stage, UNIT } from "./iso.ts";
import { grow } from "./beat.ts";
import { ACCENT } from "./model.ts";
import { said } from "./text.ts";
import "./film.css";

const SVG = "http://www.w3.org/2000/svg";
const PLINTH = "#e6e6e2";
const DARK = "#3a3a3c";
const MORPH = 520;
const EASE = "cubic-bezier(0.2, 0.8, 0.3, 1)";

/** The screen; the film fills its left face. */
const SCREEN = { x: 4, y: 1, z: 1.6, w: 16.8, d: 0.8, h: 9.8, tone: DARK };
const BEZEL = 0.4;

/** The film on the face: corner, across, down. */
function face(): [number, number][] {
  const x = SCREEN.x + BEZEL;
  const y = SCREEN.y + SCREEN.d;
  const z = SCREEN.z + SCREEN.h - BEZEL;
  const corner = project(x, y, z);
  const across = project(x + 16, y, z);
  const down = project(x, y, z - 9);
  return [corner, [across[0] - corner[0], across[1] - corner[1]], [down[0] - corner[0], down[1] - corner[1]]];
}

/** A play triangle on a block's left face. */
function notch(g: SVGGElement, x: number, y: number, z: number, w: number, h: number): void {
  const p = (dx: number, dz: number) => project(x + dx * w, y, z + dz * h).map((v) => v.toFixed(2)).join(",");
  const tri = document.createElementNS(SVG, "polygon");
  tri.setAttribute("points", [p(0.32, 0.8), p(0.32, 0.2), p(0.74, 0.5)].join(" "));
  tri.setAttribute("class", "notch");
  g.append(tri);
}

function scene(): { svg: SVGSVGElement; prop: SVGGElement } {
  const plinth = block({ x: 0, y: 0, w: 22, d: 5.2, h: 0.8, tone: PLINTH });
  const feet = [5.5, 18.5].map((x) => block({ x, y: 1.1, z: 0.8, w: 1, d: 0.6, h: 0.8, tone: DARK }));
  const screen = block(SCREEN);
  const spec = { x: 0.6, y: 2.4, z: 0.8, w: 2.4, d: 2.2, h: 1.6, tone: ACCENT };
  const prop = block(spec);
  notch(prop, spec.x, spec.y + spec.d, spec.z, spec.w, spec.h);
  grow([[plinth], [...feet, screen], [prop]]);
  const svg = stage([plinth, ...feet, screen, prop]);
  svg.classList.add("grow");
  svg.setAttribute("aria-hidden", "true");
  svg.style.setProperty("--step", `${UNIT * 0.5}px`);
  svg.style.setProperty("--ratio", String(svg.viewBox.baseVal.width / svg.viewBox.baseVal.height));
  return { svg, prop };
}

/** A screen on a plinth that becomes a player. */
export function film(): HTMLElement {
  const section = document.createElement("section");
  section.id = "film";
  section.className = "film";
  const { svg, prop } = scene();
  const land = prop.style.getPropertyValue("--land");
  section.style.setProperty("--shown", land);

  const host = document.createElement("div");
  host.className = "film-stage";
  const video = document.createElement("video");
  video.src = "/film/ely-office.mp4";
  video.poster = "/film/poster.jpg";
  video.preload = "none";
  video.playsInline = true;
  const play = said("button", "filmPlay");
  play.className = "film-play";
  host.append(svg, video, play);
  const line = said("p", "filmLine");
  line.id = "film-line";
  video.setAttribute("aria-labelledby", line.id);
  section.append(host, line);

  const reduced = matchMedia("(prefers-reduced-motion: reduce)");
  let flat = false;
  let onFace = "none";
  let morph: Animation | undefined;

  const layout = () => {
    const box = host.getBoundingClientRect();
    const art = svg.getBoundingClientRect();
    const vb = svg.viewBox.baseVal;
    const k = art.width / vb.width;
    const w = Math.floor(Math.min((box.width * 16) / (16 + 2 * BEZEL), (box.height * 16) / (9 + 2 * BEZEL)));
    const h = Math.round((w * 9) / 16);
    const left = (box.width - w) / 2;
    const top = (box.height - h) / 2;
    Object.assign(video.style, { width: `${w}px`, height: `${h}px`, left: `${left}px`, top: `${top}px` });
    video.style.setProperty("--bezel", `${(w * BEZEL) / 16}px`);
    const [corner, across, down] = face();
    const ox = art.left - box.left + (corner[0] - vb.x) * k - left;
    const oy = art.top - box.top + (corner[1] - vb.y) * k - top;
    onFace = `matrix(${(across[0] * k) / w}, ${(across[1] * k) / w}, ${(down[0] * k) / h}, ${(down[1] * k) / h}, ${ox}, ${oy})`;
    if (!flat && !morph) video.style.transform = onFace;
    const p = prop.getBoundingClientRect();
    Object.assign(play.style, {
      left: `${p.left - box.left}px`,
      top: `${p.top - box.top}px`,
      width: `${p.width}px`,
      height: `${p.height}px`,
    });
  };

  const move = (to: string) => {
    const from = getComputedStyle(video).transform;
    morph?.cancel();
    video.style.transform = to;
    if (reduced.matches) return;
    morph = video.animate([{ transform: from }, { transform: to }], { duration: MORPH, easing: EASE });
    const own = morph;
    own.onfinish = () => {
      if (morph !== own) return;
      morph = undefined;
      if (!flat) video.style.transform = onFace;
    };
  };

  const open = () => {
    if (flat) return;
    flat = true;
    section.classList.add("flat");
    video.controls = true;
    move("none");
    host.scrollIntoView({ block: "center", behavior: reduced.matches ? "instant" : "smooth" });
    video.focus({ preventScroll: true });
    video.play().catch((error: unknown) => {
      if (error instanceof DOMException && error.name === "AbortError") return;
      throw error;
    });
  };

  const close = () => {
    if (!flat) return;
    flat = false;
    section.classList.remove("flat");
    video.pause();
    video.controls = false;
    move(onFace);
    play.focus({ preventScroll: true });
  };

  play.addEventListener("click", open);
  video.addEventListener("click", () => {
    if (!flat) open();
  });
  video.addEventListener("ended", close);
  document.addEventListener("keydown", (e) => {
    if (flat && e.key === "Escape") close();
  });

  const lift = (up: boolean, press = false) => {
    prop.classList.toggle("lifted", up);
    prop.classList.toggle("pressed", up && press);
  };
  play.addEventListener("pointerenter", () => lift(true));
  play.addEventListener("pointerleave", () => lift(play.matches(":focus-visible")));
  play.addEventListener("focus", () => lift(true));
  play.addEventListener("blur", () => lift(false));
  play.addEventListener("pointerdown", () => lift(true, true));
  play.addEventListener("pointerup", () => lift(true));

  new ResizeObserver(layout).observe(host);
  new IntersectionObserver(
    (entries, seen) => {
      if (!entries.some((e) => e.isIntersecting)) return;
      svg.classList.add("seen");
      section.classList.add("seen");
      video.preload = "metadata";
      seen.disconnect();
    },
    { rootMargin: "50% 0px" },
  ).observe(section);
  return section;
}
