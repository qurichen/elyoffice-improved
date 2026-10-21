// Renders the opening's global frames to PNG, manifest and preview.
import { spawnSync } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "../scenes/node_modules/playwright/index.mjs";
import { createServer } from "vite";

const here = dirname(fileURLToPath(import.meta.url));
const site = resolve(here, "../..");
const extra = resolve(here, "../out/extra");
const dir = join(extra, "opening");
const PAGES = 6;

const server = await createServer({ root: site, configFile: false, logLevel: "error", server: { port: 0 } });
await server.listen();
const url = `${server.resolvedUrls.local[0]}video/opening/index.html`;
const browser = await chromium.launch();

/** A film-sized page; any page error stops the run. */
async function open() {
  const page = await browser.newPage({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 });
  page.on("pageerror", (e) => {
    throw e;
  });
  await page.goto(url, { waitUntil: "networkidle" });
  return page;
}

const pages = await Promise.all(Array.from({ length: PAGES }, open));
const { first, end } = await pages[0].evaluate(() => ({ first: window.opening.first, end: window.opening.end }));
const only = process.argv.slice(2).map(Number);
const frames = only.length ? only : Array.from({ length: end - first }, (_, i) => first + i);
if (!only.length) rmSync(dir, { recursive: true, force: true });
mkdirSync(dir, { recursive: true });

const queue = [...frames];
await Promise.all(
  pages.map(async (page) => {
    for (let f = queue.shift(); f !== undefined; f = queue.shift()) {
      await page.evaluate((f) => window.opening.render(f), f);
      await page.screenshot({ path: join(dir, `${String(f).padStart(5, "0")}.png`) });
    }
  }),
);
await browser.close();
await server.close();

if (!only.length) {
  const manifest = { name: "opening", first, count: end - first, fps: 30, kind: "base" };
  writeFileSync(join(dir, "manifest.json"), JSON.stringify(manifest, null, 1));
  const args = ["-y", "-v", "error", "-framerate", "30", "-start_number", String(first), "-i", join(dir, "%05d.png")];
  args.push("-c:v", "libx264", "-preset", "slow", "-crf", "16", "-pix_fmt", "yuv420p", "-movflags", "+faststart");
  const run = spawnSync("ffmpeg", [...args, join(extra, "opening.mp4")], { stdio: "inherit" });
  if (run.status !== 0) throw new Error("ffmpeg failed");
  console.log(`opening: frames ${first}..${end - 1} (${end - first})`);
}
