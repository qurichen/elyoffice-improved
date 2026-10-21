#!/usr/bin/env bash
# Usage: shot.sh [--id] <pid> [out.png]; macOS only.
set -euo pipefail

[ "$(uname)" = Darwin ] || { echo "shot: needs macOS" >&2; exit 2; }
id_only=
if [ "${1:-}" = --id ]; then
  id_only=1
  shift
fi
pid=${1:?pid}

id=$(osascript -l JavaScript - "$pid" <<'JS'
ObjC.import('CoreGraphics');
function run(argv) {
  const pid = parseInt(argv[0]);
  const all = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0)));
  const mine = all.filter(w => w.kCGWindowOwnerPID === pid && w.kCGWindowLayer === 0);
  return mine.length ? String(mine[0].kCGWindowNumber) : '';
}
JS
)
[ -n "$id" ] || { echo "shot: process $pid has no on-screen window" >&2; exit 1; }
if [ -n "$id_only" ]; then
  echo "$id"
  exit 0
fi
out=${2:?out.png}
screencapture -x -o -l"$id" "$out"
[ -s "$out" ] || { echo "shot: screencapture wrote nothing for window $id" >&2; exit 1; }
