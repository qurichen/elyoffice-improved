// Page entry: renders one scene frame to PNG data.
import { SCENES } from "./scenes.js";

const sf = new FontFace("Ely SF", "url(/SFNS.ttf)", { weight: "1 1000" });
document.fonts.add(await sf.load());
await document.fonts.load('300 20px "PingFang SC"', "六");

const big = document.createElement("canvas");
big.width = 3840;
big.height = 2160;
const out = document.createElement("canvas");
out.width = 1920;
out.height = 1080;

/** Global frame `f` of scene `name`, as base64 PNG. */
function render(name, f) {
  const scene = SCENES.find((s) => s.name === name);
  if (!scene) throw new Error(`unknown scene ${name}`);
  const ctx = big.getContext("2d");
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, big.width, big.height);
  ctx.setTransform(2, 0, 0, 2, 0, 0);
  scene.draw(ctx, f / 30);
  const small = out.getContext("2d");
  small.clearRect(0, 0, 1920, 1080);
  small.imageSmoothingEnabled = true;
  small.imageSmoothingQuality = "high";
  small.drawImage(big, 0, 0, 1920, 1080);
  return out.toDataURL("image/png").split(",")[1];
}

window.type = { scenes: SCENES.map(({ draw, ...s }) => s), render };
