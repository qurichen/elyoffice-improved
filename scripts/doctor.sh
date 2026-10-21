#!/usr/bin/env bash
# Tool versions. Exit 1 on missing or broken tools.
set -uo pipefail

failed=0
check() { # tool version-args...
  local tool=$1 out
  shift
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "MISSING $tool"
    failed=1
  elif ! out=$("$tool" "$@" 2>&1); then
    echo "BROKEN  $tool: ${out%%$'\n'*}"
    failed=1
  else
    printf '%-10s %s\n' "$tool" "${out%%$'\n'*}"
  fi
}

check rustc --version
check cargo --version
check node --version
check pnpm --version
check cmake --version
check ninja --version
check pkg-config --version
check git --version
check git-lfs version
check clang --version
check python3 --version

exit "$failed"
