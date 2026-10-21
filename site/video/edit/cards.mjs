// Renders the letterbox frame and caption overlays as PNGs.
import { mkdirSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const require = createRequire(resolve(here, "../scenes/package.json"));
const { chromium } = require("playwright");
const plan = JSON.parse(readFileSync(join(here, "plan.json"), "utf8"));
const out = resolve(here, "../out/edit/cards");
mkdirSync(out, { recursive: true });

const { x, y, w, h } = plan.window;
const css = `
  html, body { margin: 0; width: 1920px; height: 1080px; background: transparent; }
  .hole { position: absolute; left: ${x}px; top: ${y}px; width: ${w}px; height: ${h}px; border-radius: 10px;
    box-shadow: 0 0 0 1px rgba(0,0,0,.08), 0 0 0 3000px #fbfbfa; }
  .cap { position: absolute; left: ${x}px; right: ${1920 - x - w}px; top: ${y + h}px; bottom: 0;
    display: flex; align-items: center; gap: 28px; color: #1d1d1f; -webkit-font-smoothing: antialiased; }
  .name { font: 700 30px/1 "SF Pro Display", system-ui; letter-spacing: -0.01em; color: #3d7fe5; min-width: 150px; }
  .body { display: flex; flex-direction: column; gap: 8px; }
  .en { font: 300 30px/1.05 "SF Pro Display", system-ui; letter-spacing: -0.005em; color: #1d1d1f; }
  .zh { font: 300 22px/1.1 "PingFang SC", system-ui; color: #6e6e73; letter-spacing: 0.04em; }
  .count { margin-left: auto; font: 400 15px/1 "SF Mono", ui-monospace, monospace; color: #9a9a9e; letter-spacing: 0.08em; }
`;

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 });

/** Screenshots one body of markup with transparent ground. */
async function shot(html, file) {
  await page.setContent(`<!doctype html><meta charset="utf-8"><style>${css}</style>${html}`);
  await page.evaluate(() => document.fonts.ready);
  await page.screenshot({ path: join(out, file), omitBackground: true });
}

await shot(`<div class="hole"></div>`, "frame.png");
for (const [i, s] of plan.sections.entries()) {
  const count = `${String(i + 1).padStart(2, "0")} / ${String(plan.sections.length).padStart(2, "0")}`;
  const body = `<div class="body"><div class="en">${s.en}</div><div class="zh">${s.zh}</div></div>`;
  await shot(`<div class="cap"><div class="name">${s.title}</div>${body}<div class="count">${count}</div></div>`, `${s.app}.png`);
}
await browser.close();
