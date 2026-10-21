#!/usr/bin/env bash
# Each app opens, draws a frame, quits, and logs.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
cargo build -q --workspace --manifest-path "$root/Cargo.toml"
for entry in ElyDoc:ely-doc ElyTable:ely-table ElyShow:ely-show ElyPDF:ely-pdf ElyMail:ely-mail ElyTree:ely-tree ElyMark:ely-mark ElyAgent:ely-agent; do
  name=${entry%%:*}
  slug=${entry##*:}
  out=$(ELY_SMOKE=1 HOME="$work" XDG_DATA_HOME="$work/data" perl -e 'alarm 60; exec @ARGV' "$root/target/debug/$slug" 2>"$work/$slug.err") ||
    fail "$slug did not exit cleanly: $(tail -n 5 "$work/$slug.err")"
  [ "$out" = "ready $name" ] || fail "$slug printed '$out', expected 'ready $name'"
  log=$(find "$work" -path "*/ElyOffice/$slug/ely.log" | head -n 1)
  [ -n "$log" ] || fail "$slug wrote no ElyOffice/$slug/ely.log"
  grep -q "$name $version started" "$log" || fail "$slug log lacks its start line"
  grep -q "$name window open" "$log" || fail "$slug log lacks 'window open'"
done
echo "apps: 8 of 8 open a window, draw, and quit"
