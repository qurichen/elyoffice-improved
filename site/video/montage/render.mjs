// Renders the montage: frames, manifest and preview MP4.
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "../scenes/node_modules/playwright/index.mjs";
import { createServer } from "vite";

const here = dirname(fileURLToPath(import.meta.url));
const site = resolve(here, "../..");
const extra = resolve(here, "../out/extra");
const out = join(extra, "montage");
const PAGES = 4;

function ffmpeg(args) {
  const run = spawnSync("ffmpeg", ["-y", "-v", "error", ...args], { stdio: "inherit" });
  if (run.status !== 0) throw new Error(`ffmpeg failed: ${args.join(" ")}`);
}

const server = await createServer({ root: site, configFile: false, logLevel: "error", server: { port: 0 } });
await server.listen();
const url = `${server.resolvedUrls.local[0]}video/montage/index.html`;
const browser = await chromium.launch({ args: ["--use-angle=metal", "--ignore-gpu-blocklist"] });
const recs = mkdtempSync(join(tmpdir(), "montage-"));

/** A page whose rec/ requests read the extracted frames. */
async function open() {
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 });
  page.on("pageerror", (e) => {
    throw e;
  });
  page.on("console", (m) => {
    if (m.type() === "error") throw new Error(m.text());
  });
  await page.route("**/rec/**", (route) => {
    const path = new URL(route.request().url()).pathname.split("/rec/")[1];
    route.fulfill({ body: readFileSync(join(recs, path)), contentType: "image/jpeg" });
  });
  await page.goto(url, { waitUntil: "networkidle" });
  await page.waitForFunction(() => window.montage);
  return page;
}

const pages = await Promise.all(Array.from({ length: PAGES }, open));
const { FIRST, COUNT, apps } = await pages[0].evaluate(() => {
  const { FIRST, COUNT, apps } = window.montage;
  return { FIRST, COUNT, apps };
});

for (const { rec, from } of apps) {
  mkdirSync(join(recs, rec));
  const src = resolve(here, `../out/rec/${rec}.mp4`);
  const args = ["-ss", String(from), "-i", src, "-frames:v", String(COUNT), "-vf", "fps=30,scale=1280:720:flags=lanczos"];
  ffmpeg([...args, "-q:v", "2", join(recs, rec, "%05d.jpg")]);
}

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
const faces = [];
const queue = Array.from({ length: COUNT }, (_, i) => FIRST + i);
await Promise.all(
  pages.map(async (page) => {
    for (let f = queue.shift(); f !== undefined; f = queue.shift()) {
      const face = await page.evaluate((f) => window.montage.render(f), f);
      await page.screenshot({ path: join(out, `${String(f).padStart(5, "0")}.png`) });
      faces.push({ frame: f, ...face });
    }
  }),
);
faces.sort((a, b) => a.frame - b.frame);
const manifest = { name: "montage", first: FIRST, count: COUNT, fps: 30, kind: "base", faces };
writeFileSync(join(out, "manifest.json"), JSON.stringify(manifest, null, 1));
await browser.close();
await server.close();
rmSync(recs, { recursive: true, force: true });

const music = resolve(here, "../out/music.wav");
const video = ["-framerate", "30", "-start_number", String(FIRST), "-i", join(out, "%05d.png")];
const audio = ["-ss", String(FIRST / 30), "-t", String(COUNT / 30), "-i", music];
const codec = ["-c:v", "libx264", "-preset", "slow", "-crf", "14", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest"];
ffmpeg([...video, ...audio, ...codec, "-movflags", "+faststart", join(extra, "montage.mp4")]);
console.log(`montage: frames ${FIRST}..${FIRST + COUNT - 1} (${COUNT})`);
