#!/usr/bin/env bash
# Settings: one app's change reaches another; bad input is refused.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
work=$(mktemp -d)
pids=()
cleanup() {
  for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done
  rm -rf "$work"
}
trap cleanup EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

cargo build -q -p ely-pdf -p ely-tree --manifest-path "$root/Cargo.toml"
for tool in ax-values ax-press click window-shade; do
  swiftc -O -o "$work/$tool" "$root/tests/scripts/$tool.swift"
done
home="$work/home"
support="$home/Library/Application Support/ElyOffice"
mkdir -p "$support"
printf '{"version":1,"appearance":"light"}' >"$support/settings.json"
launch() {
  ELY_SMOKE=hold ELY_LOCALE=en ELY_NO_DIALOG=1 HOME="$home" \
    perl -e 'alarm 300; exec @ARGV' "$root/target/debug/$1" >"$work/$1.out" 2>&1 &
  pids+=($!)
}
ax() { "$work/ax-values" "$1"; }
[ "$("$work/ax-values" $$ | head -n 1)" != untrusted ] || { echo "SKIP: needs Accessibility permission"; exit 0; }
# Waits until $1's AX tree matches $2.
seen() {
  for _ in $(seq 100); do
    ax "$1" | grep -q -- "$2" && return 0
    sleep 0.2
  done
  return 1
}
front() {
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $1) to true" >/dev/null
  sleep 0.5
}
keys() { osascript -e 'tell application "System Events"' -e "$1" -e 'end tell' >/dev/null; }
# Another process holds the lock file $1 for $2 seconds.
hold() {
  python3 -c 'import fcntl, sys, time; f = open(sys.argv[1], "a"); fcntl.flock(f, fcntl.LOCK_EX); time.sleep(float(sys.argv[2]))' "$1" "$2" &
  pids+=($!)
  sleep 0.3
}
# Clicks the field AX names $2 in app $1.
field() {
  local at
  # A page that just opened may still be moving.
  sleep 0.5
  at=$(ax "$1" | grep -m 1 "^AXTextField.*title=$2" | sed 's/.*at=//')
  [ -n "$at" ] || fail "no field $2"
  "$work/click" "${at%,*}" "${at#*,}"
  sleep 0.3
}

launch ely-pdf
pdf=${pids[${#pids[@]}-1]}
launch ely-tree
tree=${pids[${#pids[@]}-1]}
seen "$pdf" AXWindow || fail "ElyPDF opened no window"
seen "$tree" AXWindow || fail "ElyTree opened no window"
sleep 2
shade=$("$work/window-shade" "$tree")
[ "$shade" != denied ] || { echo "SKIP: needs Screen Recording permission"; exit 0; }
[ "${shade%% *}" -gt 150 ] || fail "ElyTree starts light, not at ${shade%% *}"

# ElyPDF's Dark reaches ElyTree without a restart.
front "$pdf"
keys 'keystroke "," using command down'
seen "$pdf" "title=Settings" || fail "Cmd+, opened no settings window"
"$work/ax-press" "$pdf" Dark >/dev/null
for _ in $(seq 50); do grep -q '"dark"' "$support/settings.json" && break; sleep 0.2; done
grep -q '"appearance": "dark"' "$support/settings.json" || fail "Dark is not saved: $(cat "$support/settings.json")"
for _ in $(seq 25); do
  shade=$("$work/window-shade" "$tree")
  [ "${shade%% *}" -lt 80 ] && break
  sleep 0.2
done
[ "${shade%% *}" -lt 80 ] || fail "ElyTree did not turn dark: ${shade%% *}; $(grep -E 'settings: changed|mode ->' "$support/ely-tree/ely.log" | tail -3)"

# The author name is saved for every app.
"$work/ax-press" "$pdf" Documents >/dev/null
seen "$pdf" "title=Author name" || fail "no author field"
field "$pdf" "Author name"
keys 'keystroke "Ana Test"'
keys 'key code 36'
for _ in $(seq 50); do grep -q 'Ana Test' "$support/settings.json" && break; sleep 0.2; done
grep -q '"author": "Ana Test"' "$support/settings.json" || fail "the author is not saved"
grep -q '"appearance": "dark"' "$support/settings.json" || fail "the author write lost the appearance"

# A name from elsewhere fills the unedited field.
python3 - "$support/settings.json" <<'PY'
import json, sys
path = sys.argv[1]
fields = json.load(open(path))
fields["author"] = "Bob"
open(path, "w").write(json.dumps(fields))
PY
seen "$pdf" "value=Bob.title=Author name" || fail "the field kept the old name"
keys 'key code 48'
sleep 1
grep -q '"author": "Bob"' "$support/settings.json" || fail "leaving the field wrote the old name back"

# A failed write is shown, not only logged.
mv "$support/.settings.lock" "$support/.settings.lock.file"
mkdir "$support/.settings.lock"
field "$pdf" "Author name"
keys 'keystroke "a" using command down'
keys 'keystroke "Carol"'
keys 'key code 36'
seen "$pdf" "was not saved" || fail "the failed write is not shown"
sleep 1
seen "$pdf" "value=Carol.title=Author name" || fail "the failed name was not kept for a retry"
grep -q '"author": "Bob"' "$support/settings.json" || fail "the failed name reached the file"
rmdir "$support/.settings.lock" && mv "$support/.settings.lock.file" "$support/.settings.lock"
keys 'key code 36'
for _ in $(seq 50); do grep -q Carol "$support/settings.json" && break; sleep 0.2; done
grep -q '"author": "Carol"' "$support/settings.json" || fail "the retry did not save"

# Changing back while a save waits: last choice stays.
hold "$support/.settings.lock" 4
keys 'keystroke "a" using command down'
keys 'keystroke "Dave"'
keys 'key code 36'
sleep 0.5
keys 'keystroke "a" using command down'
keys 'keystroke "Carol"'
keys 'key code 36'
sleep 5
grep -q '"author": "Carol"' "$support/settings.json" || fail "a waiting save overtook the last choice: $(grep author "$support/settings.json")"
seen "$pdf" "value=Carol.title=Author name" || fail "the field shows a waiting save"

# ElyTree shows its own unreadable file.
mkdir -p "$support/ely-tree"
printf '{' >"$support/ely-tree/settings.json"
front "$tree"
keys 'keystroke "," using command down'
seen "$tree" "title=Settings" || fail "ElyTree opened no settings window"
"$work/ax-press" "$tree" Git >/dev/null
seen "$tree" "title=Git program" || fail "no Git field"
seen "$tree" "cannot be read" || fail "ElyTree's unreadable file is not shown"
printf '{}' >"$support/ely-tree/settings.json"

# ElyTree refuses a program that is not Git.
field "$tree" "Git program"
keys 'keystroke "/bin/echo"'
keys 'key code 36'
seen "$tree" "is not Git 2.40 or later" || fail "a non-Git program was not refused: $(ax "$tree" | grep -i "git\|echo")"
! grep -q echo "$support/ely-tree/settings.json" 2>/dev/null || fail "the refused program was saved"

# A slow check finishing last never overwrites a later choice.
cat >"$work/slow-git" <<'SH'
#!/bin/sh
sleep 4
echo "git version 2.56.0"
SH
chmod +x "$work/slow-git"
bundled="$root/engines/git/bin/git"
[ -x "$bundled" ] || fail "no bundled Git at $bundled"
field "$tree" "Git program"
keys 'keystroke "a" using command down'
keys "keystroke \"$work/slow-git\""
keys 'key code 36'
sleep 0.5
keys 'keystroke "a" using command down'
keys "keystroke \"$bundled\""
keys 'key code 36'
sleep 6
grep -q "\"git\": \"$bundled\"" "$support/ely-tree/settings.json" || fail "the slow check won: $(cat "$support/ely-tree/settings.json")"
! ax "$tree" | grep -q "is not Git" || fail "a stale refusal stayed on screen"

# Choosing the saved Git again outdates a slow check.
field "$tree" "Git program"
keys 'keystroke "a" using command down'
keys "keystroke \"$work/slow-git\""
keys 'key code 36'
sleep 0.5
keys 'keystroke "a" using command down'
keys "keystroke \"$bundled\""
keys 'key code 36'
sleep 6
grep -q "\"git\": \"$bundled\"" "$support/ely-tree/settings.json" || fail "the slow check overwrote the saved choice"

# A Git write waiting for the lock loses.
cat >"$work/fast-git" <<'SH'
#!/bin/sh
echo "git version 2.56.0"
SH
chmod +x "$work/fast-git"
hold "$support/ely-tree/.settings.lock" 4
field "$tree" "Git program"
keys 'keystroke "a" using command down'
keys "keystroke \"$work/fast-git\""
keys 'key code 36'
sleep 1
keys 'keystroke "a" using command down'
keys "keystroke \"$bundled\""
keys 'key code 36'
sleep 5
grep -q "\"git\": \"$bundled\"" "$support/ely-tree/settings.json" || fail "a waiting Git write overtook the last choice"

# An unreadable file is shown, and never overwritten.
printf '{' >"$support/settings.json"
front "$pdf"
seen "$pdf" "cannot be read" || fail "the unreadable file is not shown: $(grep settings "$support/ely-pdf/ely.log" | tail -n 3)"
"$work/ax-press" "$pdf" General >/dev/null
"$work/ax-press" "$pdf" Light >/dev/null
sleep 1
[ "$(cat "$support/settings.json")" = "{" ] || fail "the unreadable file was overwritten"
# The main window reopens where it was left.
printf '{"version":1}' >"$support/settings.json"
osascript -e "tell application \"System Events\" to set position of (first window of (first process whose unix id is $pdf) whose name is \"ElyPDF\") to {120, 140}" >/dev/null
for _ in $(seq 50); do grep -q '"x": 120' "$support/ely-pdf/settings.json" 2>/dev/null && break; sleep 0.2; done
grep -q '"x": 120' "$support/ely-pdf/settings.json" || fail "the window place is not saved: $(cat "$support/ely-pdf/settings.json" 2>/dev/null)"
kill "$pdf"
wait "$pdf" 2>/dev/null || true
launch ely-pdf
pdf=${pids[${#pids[@]}-1]}
seen "$pdf" "AXWindow.*title=ElyPDF" || fail "ElyPDF did not reopen"
place=$(osascript -e "tell application \"System Events\" to get position of (first window of (first process whose unix id is $pdf))")
[ "$place" = "120, 140" ] || fail "ElyPDF reopened at $place"
echo "ok: settings"
