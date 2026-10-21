// Cubes and flat text on a 2× canvas.

/** Film clock: 112 BPM. */
export const BEAT = 60 / 112;
export const BAR = 4 * BEAT;
export const SIX = BEAT / 4;

/** Start of 1-based bar `b`, in seconds. */
export const bar = (b) => (b - 1) * BAR;

/** CSS cubic-bezier timing, solved by bisection. */
function bezier(x1, y1, x2, y2) {
  const at = (a, b, u) => 3 * a * u * (1 - u) ** 2 + 3 * b * u * u * (1 - u) + u ** 3;
  return (x) => {
    if (x <= 0) return 0;
    if (x >= 1) return 1;
    let [lo, hi] = [0, 1];
    for (let i = 0; i < 40; i++) {
      const mid = (lo + hi) / 2;
      if (at(x1, x2, mid) < x) lo = mid;
      else hi = mid;
    }
    return at(y1, y2, (lo + hi) / 2);
  };
}

export const settle = bezier(0.2, 0.8, 0.3, 1);
export const glide = bezier(0.65, 0, 0.35, 1);
export const clamp = (u) => Math.min(1, Math.max(0, u));

const lin = (c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const gam = (c) => (c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055);

/** `hex` mixed with black `p` in oklab, as CSS rgb. */
function shade(hex, p) {
  const [r, g, b] = [1, 3, 5].map((i) => lin(parseInt(hex.slice(i, i + 2), 16) / 255));
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b) * (1 - p);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b) * (1 - p);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b) * (1 - p);
  const [L, M, S] = [l ** 3, m ** 3, s ** 3];
  const out = [
    4.0767416621 * L - 3.3077115913 * M + 0.2309699292 * S,
    -1.2684380046 * L + 2.6097574011 * M - 0.3413193965 * S,
    -0.0041960863 * L - 0.7034186147 * M + 1.707614701 * S,
  ];
  return `rgb(${out.map((c) => Math.round(clamp(gam(c)) * 255)).join(",")})`;
}

const tones = new Map();

/** Top, left and right fills of a tone. */
function faces(tone) {
  if (!tones.has(tone)) tones.set(tone, [tone, shade(tone, 0.09), shade(tone, 0.2)]);
  return tones.get(tone);
}

/** Orthographic view: isometric pitch, chosen yaw about z. */
export function view(yaw, cell, ox = 0, oy = 0) {
  const pitch = Math.atan(1 / Math.SQRT2);
  const [ca, sa, se] = [Math.cos(yaw), Math.sin(yaw), Math.sin(pitch)];
  const k = 1 / Math.cos(pitch);
  return {
    cell,
    ox,
    oy,
    p: (x, y, z) => [ox + (x * ca - y * sa) * k * cell, oy + ((x * sa + y * ca) * se * k - z) * cell],
    depth: (x, y, z) => x * sa + y * ca + z * Math.SQRT1_2,
  };
}

/** Screen bounds of boxes under view `v`. */
export function bounds(v, boxes) {
  const pts = boxes.flatMap((b) =>
    [0, 1].flatMap((i) => [0, 1].flatMap((j) => [0, 1].map((k) => v.p(b.x + i * b.w, b.y + j * b.d, b.z + k * b.h)))),
  );
  const xs = pts.map(([x]) => x);
  const ys = pts.map(([, y]) => y);
  return { x0: Math.min(...xs), x1: Math.max(...xs), y0: Math.min(...ys), y1: Math.max(...ys) };
}

function poly(ctx, pts) {
  ctx.beginPath();
  pts.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
  ctx.closePath();
  ctx.fill();
}

/** One box: outline, left, right, top; seams hidden. */
function cube(ctx, v, b) {
  const [x0, y0, z0, x1, y1, z1] = [b.x, b.y, b.z, b.x + b.w, b.y + b.d, b.z + b.h];
  const [top, left, right] = faces(b.tone);
  const p = v.p;
  ctx.globalAlpha = b.a ?? 1;
  if (ctx.globalAlpha >= 1) {
    ctx.fillStyle = left;
    poly(ctx, [p(x0, y1, z0), p(x1, y1, z0), p(x1, y0, z0), p(x1, y0, z1), p(x0, y0, z1), p(x0, y1, z1)]);
  }
  ctx.fillStyle = left;
  poly(ctx, [p(x0, y1, z0), p(x1, y1, z0), p(x1, y1, z1), p(x0, y1, z1)]);
  ctx.fillStyle = right;
  poly(ctx, [p(x1, y1, z0), p(x1, y0, z0), p(x1, y0, z1), p(x1, y1, z1)]);
  ctx.fillStyle = top;
  poly(ctx, [p(x0, y0, z1), p(x1, y0, z1), p(x1, y1, z1), p(x0, y1, z1)]);
  ctx.globalAlpha = 1;
}

/** Boxes back to front under view `v`. */
export function cubes(ctx, v, boxes) {
  const key = (b) => v.depth(b.x + b.w / 2, b.y + b.d / 2, b.z + b.h / 2);
  [...boxes]
    .filter((b) => (b.a ?? 1) > 0.002)
    .sort((a, b) => key(a) - key(b))
    .forEach((b) => cube(ctx, v, b));
}

/** Flat system text; CJK falls to PingFang SC. */
export function text(ctx, { s, x, y, size, weight = 200, color, a = 1, align = "center", track = 0 }) {
  if (a <= 0) return;
  ctx.globalAlpha = a;
  ctx.fillStyle = color;
  ctx.font = `${weight} ${size}px "Ely SF", "PingFang SC", sans-serif`;
  ctx.letterSpacing = `${track}px`;
  ctx.textAlign = align;
  ctx.textBaseline = "alphabetic";
  ctx.fillText(s, x, y);
  ctx.globalAlpha = 1;
}
