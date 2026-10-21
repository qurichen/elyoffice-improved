#!/usr/bin/env python3
"""Composites fx layers over a mock letterboxed recording for review."""
import io
import json
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from PIL import Image

HERE = Path(__file__).resolve().parent
OUT = HERE.parent / "out"
PLAN = json.loads((HERE.parent / "edit" / "plan.json").read_text())
FPS = 30
BEAT = 60 / 112
STARTS = {"doc": 32, "table": 48, "show": 64, "pdf": 80, "tree": 96, "mark": 112}


def lerp(points, x):
    """Piecewise linear through points, extended past both ends."""
    i = 0
    while i < len(points) - 2 and x > points[i + 1][0]:
        i += 1
    (x0, y0), (x1, y1) = points[i], points[i + 1]
    return y0 + (x - x0) * (y1 - y0) / (x1 - x0)


def source(app, frame):
    """Recording file and time shown at a global frame."""
    pieces = next(s for s in PLAN["sections"] if s["app"] == app)["pieces"]
    beat = frame / FPS / BEAT - STARTS[app]
    piece = pieces[0]
    for p in pieces[1:]:
        if beat >= p["anchors"][0][0]:
            piece = p
    return OUT / "rec" / f"{piece['src']}.mp4", lerp(piece["anchors"], beat)


def grab(path, t):
    png = subprocess.run(
        ["ffmpeg", "-v", "error", "-ss", f"{max(t, 0):.3f}", "-i", str(path), "-frames:v", "1",
         "-f", "image2pipe", "-vcodec", "png", "-"],
        check=True, capture_output=True).stdout
    return Image.open(io.BytesIO(png)).convert("RGB")


def layer(name, frame):
    path = OUT / "extra" / name / f"{frame:05d}.png"
    return Image.open(path).convert("RGBA") if path.exists() else None


def compose(app, frame, rect):
    """One mock frame: recording through the view, cards, then layers."""
    win = PLAN["window"]
    x, y, w, h = rect
    rec = grab(*source(app, frame)).crop((round(x), round(y), round(x + w), round(y + h)))
    canvas = Image.new("RGBA", (1920, 1080), "#fbfbfa")
    canvas.paste(rec.resize((win["w"], win["h"]), Image.LANCZOS), (win["x"], win["y"]))
    for card in (OUT / "edit" / "cards" / "frame.png", OUT / "edit" / "cards" / f"{app}.png"):
        canvas.alpha_composite(Image.open(card).convert("RGBA"))
    for name in ("fx-grid", f"{app}-fx"):
        top = layer(name, frame)
        if top is not None:
            canvas.alpha_composite(top)
    return canvas.convert("RGB")


def main():
    app, target = sys.argv[1], Path(sys.argv[2])
    picks = [int(f) for f in sys.argv[3:]]
    track = json.loads((OUT / "extra" / f"{app}-fx" / "view.json").read_text())
    first, rects = track["first"], track["rects"]
    frames = picks or list(range(first, first + len(rects)))
    target.mkdir(parents=True, exist_ok=True)

    def one(frame):
        compose(app, frame, rects[frame - first]).save(target / f"{app}-{frame:05d}.png")

    with ThreadPoolExecutor(8) as pool:
        list(pool.map(one, frames))
    if not picks:
        subprocess.run(
            ["ffmpeg", "-v", "error", "-y", "-framerate", str(FPS), "-start_number", str(first),
             "-i", str(target / f"{app}-%05d.png"), "-c:v", "libx264", "-crf", "18",
             "-pix_fmt", "yuv420p", str(target / f"{app}-mock.mp4")],
            check=True)


if __name__ == "__main__":
    main()
