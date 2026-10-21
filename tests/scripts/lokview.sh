#!/usr/bin/env bash
# A live LokView takes IME, keys, mouse and paste.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
lok=${ELY_LOK_DIR:-$root/engines/lo-src/instdir/LibreOffice.app/Contents/Frameworks}
[ -d "$lok" ] || { echo "SKIP: no LOK build at $lok"; exit 0; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

cargo build -q -p ely-lok --bins --manifest-path "$root/Cargo.toml"
cargo build -q -p ely-lok --features view --example lokview --manifest-path "$root/Cargo.toml"
out=$(ELY_SMOKE=hold ELY_LOK_DIR="$lok" HOME="$work" XDG_DATA_HOME="$work/data" \
  perl -e 'alarm 120; exec @ARGV' "$root/target/debug/examples/lokview" 2>"$work/err") ||
  fail "the probe did not exit cleanly: $(tail -n 5 "$work/err")"
grep -q "probe: focused=true" <<<"$out" || fail "the clicks did not keep focus: $out"
if [ -n "${ELY_CLIPBOARD_TEST:-}" ]; then
  grep -q 'probe: copied=Some(Html' <<<"$out" || fail "the copy left no html: $out"
  want="probe: paragraphs=你好Hello 世界X|Z|你好Hello 世界X|Z|"
else
  want="probe: paragraphs=你好Hello 世界X|Z"
fi
grep -qxF "$want" <<<"$out" || fail "expected '$want', got: $out"
! grep -q "tile paint failed\|input failed\|panicked" "$work/err" || fail "errors: $(grep -m3 'failed\|panicked' "$work/err")"
swiftc -O -o "$work/axv" "$root/tests/scripts/ax-values.swift"
if [ "$("$work/axv" $$)" = untrusted ]; then
  echo "SKIP: a11y bridge check needs Accessibility permission"
else
  ax_check() {
    ELY_PROBE_AX=$1 ELY_SMOKE=hold ELY_LOK_DIR="$lok" HOME="$work" XDG_DATA_HOME="$work/data" \
      perl -e 'alarm 120; exec @ARGV' "$root/target/debug/examples/lokview" >"$work/ax.out" 2>"$work/ax.err" &
    probe=$!
    for _ in $(seq 120); do grep -q "probe: ax hold" "$work/ax.out" 2>/dev/null && break; sleep 0.5; done
    grep -q "probe: ax hold" "$work/ax.out" || fail "the $1 AX probe never held: $(tail -n 3 "$work/ax.err")"
    seen=
    for _ in 1 2 3 4 5; do
      seen=$("$work/axv" "$probe")
      grep -qF "AXTextArea	value=$2	" <<<"$seen" && break
      sleep 1
    done
    wait "$probe" || fail "the $1 AX probe did not exit cleanly"
    grep -qF "AXTextArea	value=$2	" <<<"$seen" || fail "the $1 AX tree lacks '$2': $seen"
  }
  ax_check initial "Hello 世界"
  ax_check edited Z
fi
echo "ok: lokview"
