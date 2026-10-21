#!/usr/bin/env bash
# Shared helpers for the per-app recording scripts.
set -euo pipefail

video=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
root=$(cd "$video/../.." && pwd)
out=$video/out
bin=$out/bin
demo=$out/demo
check=${ELY_REC_CHECK:-$out/check}
inputsource=${ELY_INPUTSOURCE:?set ELY_INPUTSOURCE to the input source switcher}
# Window place in screen points.
win_x=120
win_y=120
win_w=1600
win_h=900
app_pid=
app_name=
cap_pid=

mkdir -p "$bin" "$out/rec" "$out/logs" "$check"

tools() {
  local s
  for s in ax-values ax-press ax-frames; do
    [ "$bin/$s" -nt "$root/tests/scripts/$s.swift" ] || swiftc -O -o "$bin/$s" "$root/tests/scripts/$s.swift"
  done
  [ "$bin/mouse" -nt "$video/record/mouse.swift" ] || swiftc -O -o "$bin/mouse" "$video/record/mouse.swift"
}

content() {
  python3 "$video/demo/make.py" "$demo"
  swift "$video/demo/guide.swift" "$demo/Coastal Trails Guide.pdf"
}

# Starts app $1 with files; fresh HOME, light, English.
launch() {
  local app=$1
  shift
  app_name=$app
  local home=$out/home/$app
  rm -rf "$home"
  local data="$home/Library/Application Support/ElyOffice/$app"
  mkdir -p "$data"
  local panels=
  [ -z "${PANEL_LEFT:-}" ] || panels=",\"panels\":{\"left\":$PANEL_LEFT}"
  printf '{"version":1,"window":{"mode":"windowed","x":%s,"y":%s,"width":%s,"height":%s}%s}\n' \
    "$win_x" "$win_y" "$win_w" "$win_h" "$panels" >"$data/settings.json"
  env HOME="$home" ELY_APPEARANCE=light ELY_LOCALE=en ELY_NO_DIALOG=1 ELY_LOG=info ELY_REDUCE_MOTION=0 \
    ${EXTRA_ENV:-} "$root/target/release/$app" "$@" >"$out/logs/$app.out" 2>"$out/logs/$app.err" &
  app_pid=$!
}

# Waits up to $2 seconds for log text $1.
wait_log() {
  local i
  for i in $(seq $((${2:-30} * 4))); do
    grep -q -- "$1" "$out/logs/$app_name.err" 2>/dev/null && return 0
    kill -0 "$app_pid" 2>/dev/null || { echo "app quit: $(tail -n 5 "$out/logs/$app_name.err")" >&2; return 1; }
    sleep 0.25
  done
  echo "timed out waiting for: $1" >&2
  return 1
}

# Center "x y" of the first AX line matching $1.
where() {
  local i at
  for i in $(seq 20); do
    at=$("$bin/ax-values" "$app_pid" | grep -F -- "$1" | grep -oE 'at=[0-9]+,[0-9]+' | head -n 1 | sed 's/at=//; s/,/ /' || true)
    [ -n "$at" ] && { echo "$at"; return 0; }
    sleep 0.25
  done
  echo "no AX element: $1" >&2
  return 1
}

front() {
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $app_pid) to true" >/dev/null
}

keys() { osascript -e "tell application \"System Events\" to $1" >/dev/null; }

# Types $1 at about twelve characters a second.
typed() {
  local text=$1 i c
  local script='tell application "System Events"'
  for ((i = 0; i < ${#text}; i++)); do
    c=${text:i:1}
    case $c in
      '"') c='\"' ;;
      '\') c='\\' ;;
    esac
    script+=$'\n'"keystroke \"$c\""$'\n'"delay $(awk -v r=$RANDOM 'BEGIN { printf "%.3f", 0.065 + (r % 40) / 1000 }')"
  done
  osascript -e "$script"$'\nend tell' >/dev/null
}

pause() { sleep "${1:-0.6}"; }
mouse() { "$bin/mouse" "$@"; }

# Window frame in points, as "x y w h".
frame() {
  osascript -l JavaScript - "$app_pid" <<'JS'
ObjC.import('CoreGraphics');
function run(argv) {
  const all = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0)));
  const w = all.filter(w => w.kCGWindowOwnerPID === parseInt(argv[0]) && w.kCGWindowLayer === 0)[0];
  const b = w.kCGWindowBounds;
  return [b.X, b.Y, b.Width, b.Height].join(' ');
}
JS
}

# Saves the window as $1.png in the check folder.
snap() {
  local id
  id=$("$root/tests/scripts/shot.sh" --id "$app_pid")
  screencapture -x -o -l"$id" "$check/$1.png"
}

# First x (points, window-relative) on row $2 brighter than $3.
first_bright() {
  python3 - "$check/$1.png" "$2" "$3" "${4:-0}" <<'PY'
import sys
from PIL import Image
im = Image.open(sys.argv[1]).convert("L")
scale = im.size[0] / 1600
y, level, start = int(float(sys.argv[2]) * scale), int(sys.argv[3]), int(float(sys.argv[4]) * scale)
print(next(round(x / scale) for x in range(start, im.size[0]) if im.getpixel((x, y)) >= level))
PY
}

# Captures the window, or a given region in points.
record_start() {
  local x y w h
  if [ $# -eq 5 ]; then
    x=$2 y=$3 w=$4 h=$5
  else
    read -r x y w h < <(frame)
    [ "$w" = "$win_w" ] && [ "$h" = "$win_h" ] || echo "note: window is ${w}x${h}" >&2
  fi
  rm -f "$out/rec/$1.mov"
  cap_name=$1
  ffmpeg -hide_banner -loglevel error -f avfoundation -capture_cursor 1 -framerate 60 -pixel_format nv12 -i "0:none" \
    -vf "crop=$((w * 2)):$((h * 2)):$((x * 2)):$((y * 2))" -c:v h264_videotoolbox -b:v 80M -r 60 \
    "$out/rec/$1.mov" </dev/null &
  cap_pid=$!
  t0=$(now)
  : >"$out/rec/$1.txt"
  sleep 1.2
}

now() { perl -MTime::HiRes=time -e 'printf "%.2f\n", time'; }

# Notes a key moment, in clip seconds, for the editor.
mark() {
  awk -v a="$(now)" -v b="$t0" -v m="$1" 'BEGIN { printf "%5.1f  %s\n", a - b - 0.5, m }' >>"$out/rec/$cap_name.txt"
}

# Stops capture; encodes 1920x1080 H.264 60 fps; checks frames.
record_stop() {
  local name=$1
  sleep 0.8
  kill -INT "$cap_pid"
  wait "$cap_pid" || true
  ffmpeg -hide_banner -loglevel error -y -ss 0.5 -i "$out/rec/$name.mov" -an \
    -vf "scale=1920:1080:flags=lanczos,format=yuv420p" -c:v libx264 -preset slow -crf 16 -r 60 \
    -movflags +faststart "$out/rec/$name.mp4"
  rm -f "$out/rec/$name.mov"
  local d
  d=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$out/rec/$name.mp4")
  for at in start middle end; do
    local t
    case $at in start) t=0.2 ;; middle) t=$(awk -v d="$d" 'BEGIN { print d / 2 }') ;; end) t=$(awk -v d="$d" 'BEGIN { print d - 0.3 }') ;; esac
    ffmpeg -v error -y -ss "$t" -i "$out/rec/$name.mp4" -frames:v 1 "$check/$name-$at.png"
  done
  echo "$name: $d s"
}

quit_app() {
  [ -n "$app_pid" ] || return 0
  kill "$app_pid" 2>/dev/null || true
  wait "$app_pid" 2>/dev/null || true
  app_pid=
}

us_input() { "$inputsource" com.apple.keylayout.US; }
restore_input() { "$inputsource" com.tencent.inputmethod.wetype.pinyin; }

cleanup() {
  [ -n "$cap_pid" ] && kill -INT "$cap_pid" 2>/dev/null || true
  quit_app
  restore_input
}
trap cleanup EXIT

tools
