// The film's clock: 112 BPM; motion ends on the grid.

/** Seconds per beat and per bar. */
export const BEAT = 60 / 112;
export const BAR = 4 * BEAT;

/** Landings and fades last this long, ending on beats. */
export const LAND = 0.2;

/** CSS cubic-bezier timing, solved by bisection. */
export function bezier(x1: number, y1: number, x2: number, y2: number): (x: number) => number {
  const at = (a: number, b: number, u: number) => 3 * a * u * (1 - u) ** 2 + 3 * b * u * u * (1 - u) + u ** 3;
  return (x) => {
    if (x <= 0) return 0;
    if (x >= 1) return 1;
    let [lo, hi] = [0, 1];
    for (let i = 0; i < 50; i++) {
      const mid = (lo + hi) / 2;
      if (at(x1, x2, mid) < x) lo = mid;
      else hi = mid;
    }
    return at(y1, y2, (lo + hi) / 2);
  };
}

/** The site's landing curve, from iso.css. */
export const settle = bezier(0.2, 0.8, 0.3, 1);
export const glide = bezier(0.65, 0, 0.35, 1);

/** Progress of a move lasting `span` that ends on `beat`. */
export function toward(t: number, beat: number, span = LAND, ease = settle): number {
  return ease((t - (beat * BEAT - span)) / span);
}

/** Whether a move ending on `beat` has started by `t`. */
export function begun(t: number, beat: number, span = LAND): boolean {
  return t > beat * BEAT - span;
}
