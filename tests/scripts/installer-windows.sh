#!/usr/bin/env bash
# The Windows installer builds, keeps data, links the runtime statically.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
nsi=$root/packaging/windows/installer.nsi
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

command -v makensis >/dev/null || {
  echo "SKIP installer-windows: makensis is not installed"
  exit 0
}
printf 'MZ' >"$work/app.exe"
count=0
while IFS=$'\t' read -r slug name _; do
  out=$work/$slug-setup.exe
  makensis -V1 -DAPP_NAME="$name" -DAPP_SLUG="$slug" -DVERSION=0.1.0 -DEXE="$work/app.exe" \
    -DICON="$root/assets/icons/$slug.ico" \
    -DOUTFILE="$out" "$nsi" >"$work/makensis.log" 2>&1 || fail "$slug: $(tail -n 5 "$work/makensis.log")"
  [ "$(head -c 2 "$out")" = MZ ] || fail "$slug installer is not a Windows executable"
  count=$((count + 1))
done <"$root/packaging/apps.tsv"
[ "$count" -eq 8 ] || fail "built $count installers, expected 8"

[ "$(grep -c 'RMDir /r' "$nsi")" -eq 1 ] || fail "the uninstaller removes more than one tree"
grep -qx '  RMDir /r "$LOCALAPPDATA\\ElyOffice\\${APP_SLUG}"' "$nsi" || fail "the removed tree is not the app's own data directory"
grep -qx '  MessageBox MB_YESNO|MB_DEFBUTTON2 "$(DeleteData)" IDNO done' "$nsi" || fail "data removal is not confirmed with No as the default"
if rustup target add x86_64-pc-windows-msvc >/dev/null 2>&1; then
  mkdir -p "$work/probe/src"
  printf '[package]\nname = "probe"\nversion = "0.0.0"\nedition = "2021"\n' >"$work/probe/Cargo.toml"
  printf 'fn main() {}\n' >"$work/probe/src/main.rs"
  cfg=$(cd "$root" && cargo rustc -q --manifest-path "$work/probe/Cargo.toml" --target x86_64-pc-windows-msvc -- --print cfg 2>&1) ||
    fail "cargo rustc for the msvc target failed: $cfg"
  grep -qx 'target_feature="crt-static"' <<<"$cfg" || fail "the msvc build links the C runtime dynamically"
  echo "installer-windows: msvc target links the C runtime statically"
else
  echo "SKIP installer-windows: msvc target is not installable, so the runtime link was not checked"
fi
if makensis -V1 -DAPP_SLUG=x -DVERSION=1 -DEXE="$work/app.exe" "$nsi" >/dev/null 2>&1; then
  fail "an installer without APP_NAME built"
fi
echo "installer-windows: 8 installers build; uninstall keeps documents and asks before data"
