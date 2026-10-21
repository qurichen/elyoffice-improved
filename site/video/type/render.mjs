// Renders voxel titles to PNG frames, manifests and previews.
import { spawnSync } from "node:child_process";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { dirname, extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "../scenes/node_modules/playwright/index.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const out = resolve(here, "../out/extra");
const PAGES = 4;
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".ttf": "font/ttf" };
const SF = "/System/Library/Fonts/SFNS.ttf";

const server = createServer((req, res) => {
  const name = new URL(req.url, "http://x").pathname;
  if (name === "/SFNS.ttf") {
    res.writeHead(200, { "content-type": TYPES[".ttf"] }).end(readFileSync(SF));
    return;
  }
  const path = join(here, name);
  const type = TYPES[extname(path)];
  if (!type || !path.startsWith(here)) {
    res.writeHead(404).end();
    return;
  }
  res.writeHead(200, { "content-type": type }).end(readFileSync(path));
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const url = `http://127.0.0.1:${server.address().port}/index.html`;
const browser = await chromium.launch();

/** A page; any page error stops the run. */
async function open() {
  const page = await browser.newPage({ viewport: { width: 400, height: 200 } });
  page.on("pageerror", (e) => {
    throw e;
  });
  await page.goto(url, { waitUntil: "networkidle" });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForFunction(() => window.type);
  return page;
}

const pages = await Promise.all(Array.from({ length: PAGES }, open));
const all = await pages[0].evaluate(() => window.type.scenes);
const png = async (page, name, f) => Buffer.from(await page.evaluate(([n, f]) => window.type.render(n, f), [name, f]), "base64");

/** Each scene's frames, manifest and preview MP4. */
async function scene(page, s) {
  const dir = join(out, s.name);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  for (let f = s.first; f < s.first + s.count; f++) writeFileSync(join(dir, `${String(f).padStart(5, "0")}.png`), await png(page, s.name, f));
  const manifest = { name: s.name, first: s.first, count: s.count, fps: 30, kind: s.kind };
  writeFileSync(join(dir, "manifest.json"), JSON.stringify(manifest, null, 1));
  const input = ["-framerate", "30", "-start_number", String(s.first), "-i", join(dir, "%05d.png")];
  const paper = ["-f", "lavfi", "-i", `color=c=0xfbfbfa:s=1920x1080:r=30:d=${s.count / 30}`];
  const args = s.kind === "over" ? [...paper, ...input, "-filter_complex", "[0][1]overlay=shortest=1"] : input;
  const enc = ["-c:v", "libx264", "-preset", "slow", "-crf", "14", "-pix_fmt", "yuv420p", "-movflags", "+faststart"];
  const run = spawnSync("ffmpeg", ["-y", "-v", "error", ...args, ...enc, join(out, `${s.name}.mp4`)], { stdio: "inherit" });
  if (run.status !== 0) throw new Error(`ffmpeg failed for ${s.name}`);
  console.log(`${s.name}: ${s.kind} frames ${s.first}..${s.first + s.count - 1}`);
}

/** Stills of chosen global frames, for looking. */
async function stills(dir, frames) {
  mkdirSync(dir, { recursive: true });
  for (const f of frames) {
    const s = all.find((s) => f >= s.first && f < s.first + s.count);
    if (!s) throw new Error(`no scene at frame ${f}`);
    writeFileSync(join(dir, `${s.name}-${f}.png`), await png(pages[0], s.name, f));
  }
}

const args = process.argv.slice(2);
if (args[0] === "--stills") {
  await stills(resolve(args[1]), args.slice(2).map(Number));
} else {
  const unknown = args.filter((n) => !all.some((s) => s.name === n));
  if (unknown.length) throw new Error(`unknown scenes: ${unknown.join(", ")}`);
  const queue = all.filter((s) => !args.length || args.includes(s.name));
  await Promise.all(pages.map(async (page) => {
    for (let s = queue.shift(); s; s = queue.shift()) await scene(page, s);
  }));
}
await browser.close();
server.close();
