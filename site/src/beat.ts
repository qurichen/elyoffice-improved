// The film's tempo: 112 BPM, landings on sixteenths.
const BEAT = 60 / 112;
const STEP = BEAT / 4;

/** Times parts to land beat by beat, blocks on sixteenths. */
export function grow(parts: SVGGElement[][]): void {
  let beat = 1;
  for (const blocks of parts) {
    blocks.forEach((g, i) => g.style.setProperty("--land", `${(beat * BEAT + i * STEP).toFixed(3)}s`));
    beat += Math.ceil(blocks.length / 4);
  }
}
