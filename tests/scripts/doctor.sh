#!/usr/bin/env bash
# Runs doctor.sh against stub tools only.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
stubs=$(mktemp -d)
trap 'rm -rf "$stubs"' EXIT
tools="rustc cargo node pnpm cmake ninja pkg-config git git-lfs clang python3"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}
run() { PATH="$stubs" "$BASH" "$root/scripts/doctor.sh"; }

for tool in $tools; do
  printf '#!/bin/sh\necho "%s 1.0"\n' "$tool" >"$stubs/$tool"
  chmod +x "$stubs/$tool"
done

out=$(run) || fail "all tools present: doctor exited non-zero"
for tool in $tools; do
  grep -q "^$tool  *$tool 1.0$" <<<"$out" || fail "no version line for $tool: $out"
done

for tool in $tools; do
  mv "$stubs/$tool" "$stubs/$tool.off"
  if out=$(run); then fail "$tool absent: doctor exited 0"; fi
  grep -q "^MISSING $tool$" <<<"$out" || fail "no MISSING line for $tool: $out"
  mv "$stubs/$tool.off" "$stubs/$tool"
done

printf '#!/bin/sh\necho "dyld: Library not loaded" >&2\nexit 134\n' >"$stubs/ninja"
if out=$(run); then fail "ninja broken: doctor exited 0"; fi
grep -q '^BROKEN  ninja: dyld: Library not loaded$' <<<"$out" || fail "no BROKEN line for ninja: $out"

echo "doctor.sh: present, absent x11, broken pass"
